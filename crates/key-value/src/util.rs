use crate::{Error, Store, StoreManager};
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

const DEFAULT_CACHE_SIZE: usize = 256;

pub struct EmptyStoreManager;

#[async_trait]
impl StoreManager for EmptyStoreManager {
    async fn get(&self, _name: &str) -> Result<Arc<dyn Store>, Error> {
        Err(Error::NoSuchStore)
    }
}

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
        self.delegates
            .get(name)
            .ok_or(Error::NoSuchStore)?
            .get(name)
            .await
    }
}

/// Wrap each `Store` produced by the inner `StoreManager` in an asynchronous, write-behind cache.
///
/// This serves two purposes:
///
/// - Improve performance with slow and/or distant stores
///
/// - Provide a relaxed consistency guarantee vs. what a fully synchronous store provides
///
/// The latter is intended to prevent guests from coming to rely on the synchronous consistency model of an
/// existing implementation which may later be replaced with one providing a more relaxed, asynchronous
/// (i.e. "eventual") consistency model.  See also https://www.hyrumslaw.com/ and https://xkcd.com/1172/.
///
/// This implementation provides a "read-your-writes", asynchronous consistency model such that values are
/// immediately available for reading as soon as they are written as long as the read(s) hit the same cache as the
/// write(s).  Reads and writes through separate caches (e.g. separate guest instances or separately-opened
/// references to the same store within a single instance) are _not_ guaranteed to be consistent; not only is
/// cross-cache consistency subject to scheduling and/or networking delays, a given tuple is never refreshed from
/// the backing store once added to a cache since this implementation is intended for use only by short-lived guest
/// instances.
///
/// Note that, because writes are asynchronous and return immediately, durability is _not_ guaranteed.  I/O errors
/// may occur asyncronously after the write operation has returned control to the guest, which may result in the
/// write being lost without the guest knowing.  In the future, a separate `write-durable` function could be added
/// to key-value.wit to provide either synchronous or asynchronous feedback on durability for guests which need it.
pub struct CachingStoreManager<T> {
    capacity: NonZeroUsize,
    inner: T,
}

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
            state: AsyncMutex::new(CachingStoreState {
                cache: LruCache::new(self.capacity),
                previous_task: None,
            }),
        }))
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
        self.previous_task = Some(task::spawn(async move {
            if let Some(previous_task) = previous_task {
                previous_task
                    .await
                    .map_err(|e| Error::Io(format!("{e:?}")))??
            }

            task.await
        }))
    }

    async fn flush(&mut self) -> Result<(), Error> {
        if let Some(previous_task) = self.previous_task.take() {
            previous_task
                .await
                .map_err(|e| Error::Io(format!("{e:?}")))??
        }

        Ok(())
    }
}

struct CachingStore {
    inner: Arc<dyn Store>,
    state: AsyncMutex<CachingStoreState>,
}

#[async_trait]
impl Store for CachingStore {
    async fn get(&self, key: &str) -> Result<Vec<u8>, Error> {
        // Retrieve the specified value from the cache, lazily populating the cache as necessary.

        let mut state = self.state.lock().await;

        if let Some(value) = state.cache.get(key).cloned() {
            value
        } else {
            // Flush any outstanding writes prior to reading from store.  This is necessary because we need to
            // guarantee the guest will read its own writes even if entries have been popped off the end of the LRU
            // cache prior to their corresponding writes reaching the backing store.
            state.flush().await?;

            let value = match self.inner.get(key).await {
                Ok(value) => Some(value),
                Err(Error::NoSuchKey) => None,
                e => return e,
            };

            state.cache.put(key.to_owned(), value.clone());

            value
        }
        .ok_or(Error::NoSuchKey)
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
        match self.get(key).await {
            Ok(_) => Ok(true),
            Err(Error::NoSuchKey) => Ok(false),
            Err(e) => Err(e),
        }
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
}
