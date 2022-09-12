use serde::Serialize;
use serde_json::Value;

// ValuesMap stores dynamically-typed values.
pub type ValuesMap = serde_json::Map<String, Value>;

/// ValuesMapBuilder assists in building a ValuesMap.
#[derive(Default)]
pub struct ValuesMapBuilder(ValuesMap);

impl ValuesMapBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn string(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.entry(key, value.into())
    }

    pub fn string_option(
        &mut self,
        key: impl Into<String>,
        value: Option<impl Into<String>>,
    ) -> &mut Self {
        if let Some(value) = value {
            self.0.insert(key.into(), value.into().into());
        }
        self
    }

    pub fn string_array<T: Into<String>>(
        &mut self,
        key: impl Into<String>,
        iter: impl IntoIterator<Item = T>,
    ) -> &mut Self {
        self.entry(key, iter.into_iter().map(|s| s.into()).collect::<Vec<_>>())
    }

    pub fn entry(&mut self, key: impl Into<String>, value: impl Into<Value>) -> &mut Self {
        self.0.insert(key.into(), value.into());
        self
    }

    pub fn serializable(
        &mut self,
        key: impl Into<String>,
        value: impl Serialize,
    ) -> serde_json::Result<&mut Self> {
        let value = serde_json::to_value(value)?;
        self.0.insert(key.into(), value);
        Ok(self)
    }

    pub fn build(&mut self) -> ValuesMap {
        std::mem::take(&mut self.0)
    }
}
