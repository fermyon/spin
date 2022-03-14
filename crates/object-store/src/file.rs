use std::{
    cell::RefCell,
    io::{Read, Write},
    path::Path,
};

use crate::wit::spin_object_store;
use anyhow::Context;
use cap_std::{ambient_authority, fs::OpenOptions};

pub type FileObjectStoreData = (
    FileObjectStore,
    spin_object_store::SpinObjectStoreTables<FileObjectStore>,
);

pub struct FileObjectStore {
    root: cap_std::fs::Dir,
}

impl FileObjectStore {
    pub fn new(root: impl AsRef<Path>) -> anyhow::Result<Self> {
        let root = cap_std::fs::Dir::open_ambient_dir(root, ambient_authority())
            .context("failed to open root")?;
        Ok(Self { root })
    }
}

impl From<FileObjectStore> for FileObjectStoreData {
    fn from(fos: FileObjectStore) -> Self {
        (fos, Default::default())
    }
}

impl spin_object_store::SpinObjectStore for FileObjectStore {
    type ObjectReader = RefCell<cap_std::fs::File>;
    type ObjectWriter = RefCell<Option<cap_std::fs::File>>;

    fn object_reader_read(
        &mut self,
        file: &Self::ObjectReader,
        buf: &mut [u8],
    ) -> Result<spin_object_store::Size, spin_object_store::Error> {
        let size = file.borrow_mut().read(buf).map_err(|err| err.to_string())?;
        u64::try_from(size).map_err(|err| err.to_string())
    }

    fn object_reader_size(&mut self, file: &Self::ObjectReader) -> Option<spin_object_store::Size> {
        file.borrow()
            .metadata()
            .map(|meta| meta.len())
            .map_err(|err| tracing::debug!("failed to read metadata for {:?}: {}", file, err))
            .ok()
    }

    fn object_writer_write(
        &mut self,
        file: &Self::ObjectWriter,
        buf: &[u8],
    ) -> Result<(), spin_object_store::Error> {
        match file.borrow_mut().as_mut() {
            Some(file) => file.write_all(buf).map_err(|err| err.to_string()),
            None => Err("already committed".to_string()),
        }
    }

    fn object_writer_commit(
        &mut self,
        file: &Self::ObjectWriter,
    ) -> Result<(), spin_object_store::Error> {
        if let Some(file) = file.take() {
            file.sync_all().map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    fn get_object(&mut self, key: &str) -> Result<Self::ObjectReader, spin_object_store::Error> {
        let file = self
            .root
            .open(key)
            .map_err(|err| format!("get failed for {:?}: {}", key, err))?;
        Ok(RefCell::new(file))
    }

    fn put_object(&mut self, key: &str) -> Result<Self::ObjectWriter, spin_object_store::Error> {
        let file = self
            .root
            .open_with(key, OpenOptions::new().create(true).write(true))
            .map_err(|err| err.to_string())?;
        Ok(RefCell::new(Some(file)))
    }

    fn delete_object(&mut self, key: &str) -> Result<(), spin_object_store::Error> {
        self.root.remove_file(key).map_err(|err| err.to_string())
    }
}
