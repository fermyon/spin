use redis::Commands;
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;

pub use key_value::add_to_linker;

wit_bindgen_wasmtime::export!("../../wit/ephemeral/key-value.wit");

pub struct KeyValue {
    store: Box<dyn StoreOps + Send>,
}

trait StoreOps {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, anyhow::Error>;
    fn set(&mut self, key: &str, value: &[u8]) -> Result<(), anyhow::Error>;
    fn delete(&mut self, key: &str) -> Result<(), anyhow::Error>;
}

struct RedisStore {
    client: redis::Client,
}

struct FileStore {
    db: HashMap<String, Vec<u8>>,
    path: PathBuf,
}

impl StoreOps for RedisStore {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, anyhow::Error> {
        let mut conn = self.client.get_connection()?;
        let value: Option<Vec<u8>> = conn.get(key)?;
        Ok(value)
    }

    fn set(&mut self, key: &str, value: &[u8]) -> Result<(), anyhow::Error> {
        let mut conn = self.client.get_connection()?;
        conn.set(key, value)?;
        Ok(())
    }

    fn delete(&mut self, key: &str) -> Result<(), anyhow::Error> {
        let mut conn = self.client.get_connection()?;
        conn.del(key)?;
        Ok(())
    }
}

impl FileStore {
    fn save(&self) -> Result<(), anyhow::Error> {
        let json = serde_json::to_vec(&self.db)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }
}

impl StoreOps for FileStore {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, anyhow::Error> {
        Ok(self.db.get(key).cloned())
    }

    fn set(&mut self, key: &str, value: &[u8]) -> Result<(), anyhow::Error> {
        self.db.insert(key.to_string(), value.to_vec());
        self.save()
    }

    fn delete(&mut self, key: &str) -> Result<(), anyhow::Error> {
        self.db.remove(key);
        self.save()
    }
}

impl KeyValue {
    pub fn from_file(path: PathBuf) -> Result<KeyValue, anyhow::Error> {
        let mut file = match std::fs::File::open(&path) {
            Ok(file) => file,
            Err(_) => {
                return Ok(KeyValue {
                    store: Box::new(FileStore {
                        db: HashMap::new(),
                        path,
                    }),
                })
            }
        };
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(KeyValue {
            store: Box::new(FileStore {
                db: serde_json::from_str(&contents)?,
                path,
            }),
        })
    }
}

impl KeyValue {
    pub fn from_redis(address: &str) -> Result<KeyValue, anyhow::Error> {
        Ok(KeyValue {
            store: Box::new(RedisStore {
                client: redis::Client::open(address)?,
            }),
        })
    }
}

impl key_value::KeyValue for KeyValue {
    fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, key_value::Error> {
        self.store.get(key).map_err(|_| key_value::Error::Error)
    }
    fn set(&mut self, key: &str, value: &[u8]) -> Result<(), key_value::Error> {
        self.store
            .set(key, value)
            .map_err(|_| key_value::Error::Error)
    }
    fn delete(&mut self, key: &str) -> Result<(), key_value::Error> {
        self.store.delete(key).map_err(|_| key_value::Error::Error)
    }
}

#[cfg(test)]
mod tests {
    use crate::key_value::KeyValue;
    use std::collections::HashMap;
    use std::fs::read_to_string;
    #[test]
    fn test_save_to_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut kv = super::KeyValue::from_file(temp_dir.path().join("db.json")).unwrap();
        kv.set("key", b"value").unwrap();
        assert_eq!(kv.get("key").unwrap().unwrap(), b"value");

        let file_contents = read_to_string(temp_dir.path().join("db.json")).unwrap();
        let db: HashMap<String, Vec<u8>> = serde_json::from_str(&file_contents).unwrap();
        assert_eq!(db.get("key").unwrap(), b"value");
    }

    #[test]
    fn test_delete_key() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut kv = super::KeyValue::from_file(temp_dir.path().join("db.json")).unwrap();
        kv.set("key", b"value").unwrap();
        assert_eq!(kv.get("key").unwrap().unwrap(), b"value");
        kv.delete("key").unwrap();
        assert_eq!(kv.get("key").unwrap(), None);

        let file_contents = read_to_string(temp_dir.path().join("db.json")).unwrap();
        let db: HashMap<String, Vec<u8>> = serde_json::from_str(&file_contents).unwrap();
        assert_eq!(db.get("key"), None);
    }
}
