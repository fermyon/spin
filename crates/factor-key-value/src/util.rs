use crate::{Cas, Error, Store, StoreManager, SwapError};
use lru::LruCache;
use spin_core::async_trait;
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    num::NonZeroUsize,
    sync::Arc,
};
use tokio::{
    sync::Mutex as AsyncMutex,
    task::{self, JoinHandle},
};
use tracing::Instrument;

/// A [`StoreManager`] which delegates to other `StoreManager`s based on the store label.
pub struct DelegatingStoreManager {
    delegates: HashMap<String, Arc<dyn StoreManager>>,
}

impl DelegatingStoreManager {
    pub fn new(delegates: impl IntoIterator<Item = (String, Arc<dyn StoreManager>)>) -> Self {
        let delegates = delegates.into_iter().collect();
        Self { delegates }
    }
}

#[async_trait]
impl StoreManager for DelegatingStoreManager {
    async fn get(&self, name: &str) -> Result<Arc<dyn Store>, Error> {
        match self.delegates.get(name) {
            Some(store) => store.get(name).await,
            None => Err(Error::NoSuchStore),
        }
    }

    fn is_defined(&self, store_name: &str) -> bool {
        self.delegates.contains_key(store_name)
    }

    fn summary(&self, store_name: &str) -> Option<String> {
        if let Some(store) = self.delegates.get(store_name) {
            return store.summary(store_name);
        }
        None
    }
}

/// Wrap each `Store` produced by the inner `StoreManager` in an asynchronous,
/// write-behind cache.
///
/// This serves two purposes:
///
/// - Improve performance with slow and/or distant stores
///
/// - Provide a relaxed consistency guarantee vs. what a fully synchronous store
///   provides
///
/// The latter is intended to prevent guests from coming to rely on the
/// synchronous consistency model of an existing implementation which may later
/// be replaced with one providing a more relaxed, asynchronous (i.e.
/// "eventual") consistency model.  See also <https://www.hyrumslaw.com/> and
/// <https://xkcd.com/1172/>.
///
/// This implementation provides a "read-your-writes", asynchronous consistency
/// model such that values are immediately available for reading as soon as they
/// are written as long as the read(s) hit the same cache as the write(s).
/// Reads and writes through separate caches (e.g. separate guest instances or
/// separately-opened references to the same store within a single instance) are
/// _not_ guaranteed to be consistent; not only is cross-cache consistency
/// subject to scheduling and/or networking delays, a given tuple is never
/// refreshed from the backing store once added to a cache since this
/// implementation is intended for use only by short-lived guest instances.
///
/// Note that, because writes are asynchronous and return immediately,
/// durability is _not_ guaranteed.  I/O errors may occur asynchronously after
/// the write operation has returned control to the guest, which may result in
/// the write being lost without the guest knowing.  In the future, a separate
/// `write-durable` function could be added to key-value.wit to provide either
/// synchronous or asynchronous feedback on durability for guests which need it.
pub struct CachingStoreManager<T> {
    capacity: NonZeroUsize,
    inner: T,
}

const DEFAULT_CACHE_SIZE: usize = 256;

impl<T> CachingStoreManager<T> {
    pub fn new(inner: T) -> Self {
        Self::new_with_capacity(NonZeroUsize::new(DEFAULT_CACHE_SIZE).unwrap(), inner)
    }

    pub fn new_with_capacity(capacity: NonZeroUsize, inner: T) -> Self {
        Self { capacity, inner }
    }
}

#[async_trait]
impl<T: StoreManager> StoreManager for CachingStoreManager<T> {
    async fn get(&self, name: &str) -> Result<Arc<dyn Store>, Error> {
        Ok(Arc::new(CachingStore {
            inner: self.inner.get(name).await?,
            state: Arc::new(AsyncMutex::new(CachingStoreState {
                cache: LruCache::new(self.capacity),
                previous_task: None,
            })),
        }))
    }

    fn is_defined(&self, store_name: &str) -> bool {
        self.inner.is_defined(store_name)
    }

    fn summary(&self, store_name: &str) -> Option<String> {
        self.inner.summary(store_name)
    }
}

struct CachingStoreState {
    cache: LruCache<String, Option<Vec<u8>>>,
    previous_task: Option<JoinHandle<Result<(), Error>>>,
}

impl CachingStoreState {
    /// Wrap the specified task in an outer task which waits for `self.previous_task` before proceeding, and spawn
    /// the result.  This ensures that write order is preserved.
    fn spawn(&mut self, task: impl Future<Output = Result<(), Error>> + Send + 'static) {
        let previous_task = self.previous_task.take();
        let task = async move {
            if let Some(previous_task) = previous_task {
                previous_task
                    .await
                    .map_err(|e| Error::Other(format!("{e:?}")))??
            }

            task.await
        };
        self.previous_task = Some(task::spawn(task.in_current_span()))
    }

    async fn flush(&mut self) -> Result<(), Error> {
        if let Some(previous_task) = self.previous_task.take() {
            previous_task
                .await
                .map_err(|e| Error::Other(format!("{e:?}")))??
        }

        Ok(())
    }
}

struct CachingStore {
    inner: Arc<dyn Store>,
    state: Arc<AsyncMutex<CachingStoreState>>,
}

#[async_trait]
impl Store for CachingStore {
    async fn after_open(&self) -> Result<(), Error> {
        self.inner.after_open().await
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        // Retrieve the specified value from the cache, lazily populating the cache as necessary.

        let mut state = self.state.lock().await;

        if let Some(value) = state.cache.get(key).cloned() {
            return Ok(value);
        }

        // Flush any outstanding writes prior to reading from store.  This is necessary because we need to
        // guarantee the guest will read its own writes even if entries have been popped off the end of the LRU
        // cache prior to their corresponding writes reaching the backing store.
        state.flush().await?;

        let value = self.inner.get(key).await?;

        state.cache.put(key.to_owned(), value.clone());

        Ok(value)
    }

    async fn set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        // Update the cache and spawn a task to update the backing store asynchronously.

        let mut state = self.state.lock().await;

        state.cache.put(key.to_owned(), Some(value.to_owned()));

        let inner = self.inner.clone();
        let key = key.to_owned();
        let value = value.to_owned();
        state.spawn(async move { inner.set(&key, &value).await });

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), Error> {
        // Update the cache and spawn a task to update the backing store asynchronously.

        let mut state = self.state.lock().await;

        state.cache.put(key.to_owned(), None);

        let inner = self.inner.clone();
        let key = key.to_owned();
        state.spawn(async move { inner.delete(&key).await });

        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, Error> {
        Ok(self.get(key).await?.is_some())
    }

    async fn get_keys(&self) -> Result<Vec<String>, Error> {
        // Get the keys from the backing store, remove any which are `None` in the cache, and add any which are
        // `Some` in the cache, returning the result.
        //
        // Note that we don't bother caching the result, since we expect this function won't be called more than
        // once for a given store in normal usage, and maintaining consistency would be complicated.

        let mut state = self.state.lock().await;

        // Flush any outstanding writes first in case entries have been popped off the end of the LRU cache prior
        // to their corresponding writes reaching the backing store.
        state.flush().await?;

        Ok(self
            .inner
            .get_keys()
            .await?
            .into_iter()
            .filter(|k| {
                state
                    .cache
                    .peek(k)
                    .map(|v| v.as_ref().is_some())
                    .unwrap_or(true)
            })
            .chain(
                state
                    .cache
                    .iter()
                    .filter_map(|(k, v)| v.as_ref().map(|_| k.to_owned())),
            )
            .collect::<HashSet<_>>()
            .into_iter()
            .collect())
    }

    async fn get_many(
        &self,
        keys: Vec<String>,
    ) -> anyhow::Result<Vec<(String, Option<Vec<u8>>)>, Error> {
        let mut state = self.state.lock().await;

        // Flush any outstanding writes first in case entries have been popped off the end of the LRU cache prior
        // to their corresponding writes reaching the backing store.
        state.flush().await?;

        let mut found: Vec<(String, Option<Vec<u8>>)> = Vec::new();
        let mut not_found: Vec<String> = Vec::new();
        for key in keys {
            match state.cache.get(key.as_str()) {
                Some(Some(value)) => found.push((key, Some(value.clone()))),
                _ => not_found.push(key),
            }
        }

        if !not_found.is_empty() {
            let keys_and_values = self.inner.get_many(not_found).await?;
            for (key, value) in keys_and_values {
                found.push((key.clone(), value.clone()));
                state.cache.put(key, value);
            }
        }

        Ok(found)
    }

    async fn set_many(&self, key_values: Vec<(String, Vec<u8>)>) -> anyhow::Result<(), Error> {
        let mut state = self.state.lock().await;

        for (key, value) in key_values.clone() {
            state.cache.put(key, Some(value));
        }

        self.inner.set_many(key_values).await
    }

    async fn delete_many(&self, keys: Vec<String>) -> anyhow::Result<(), Error> {
        let mut state = self.state.lock().await;

        for key in keys.clone() {
            state.cache.put(key, None);
        }

        self.inner.delete_many(keys).await
    }

    async fn increment(&self, key: String, delta: i64) -> anyhow::Result<i64, Error> {
        let mut state = self.state.lock().await;
        let counter = self.inner.increment(key.clone(), delta).await?;
        state
            .cache
            .put(key, Some(i64::to_le_bytes(counter).to_vec()));
        Ok(counter)
    }

    async fn new_compare_and_swap(
        &self,
        bucket_rep: u32,
        key: &str,
    ) -> anyhow::Result<Arc<dyn Cas>, Error> {
        let inner = self.inner.new_compare_and_swap(bucket_rep, key).await?;
        Ok(Arc::new(CompareAndSwap {
            bucket_rep,
            state: self.state.clone(),
            key: key.to_string(),
            inner_cas: inner,
        }))
    }
}

struct CompareAndSwap {
    bucket_rep: u32,
    key: String,
    state: Arc<AsyncMutex<CachingStoreState>>,
    inner_cas: Arc<dyn Cas>,
}

#[async_trait]
impl Cas for CompareAndSwap {
    async fn current(&self) -> anyhow::Result<Option<Vec<u8>>, Error> {
        let mut state = self.state.lock().await;
        state.flush().await?;
        let res = self.inner_cas.current().await;
        match res.clone() {
            Ok(value) => {
                state.cache.put(self.key.clone(), value.clone());
                state.flush().await?;
                Ok(value)
            }
            Err(err) => Err(err),
        }?;
        res
    }

    async fn swap(&self, value: Vec<u8>) -> anyhow::Result<(), SwapError> {
        let mut state = self.state.lock().await;
        state
            .flush()
            .await
            .map_err(|_e| SwapError::Other("failed flushing".to_string()))?;
        let res = self.inner_cas.swap(value.clone()).await;
        match res {
            Ok(()) => {
                state.cache.put(self.key.clone(), Some(value));
                state
                    .flush()
                    .await
                    .map_err(|_e| SwapError::Other("failed flushing".to_string()))?;
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    async fn bucket_rep(&self) -> u32 {
        self.bucket_rep
    }

    async fn key(&self) -> String {
        self.key.clone()
    }
}
