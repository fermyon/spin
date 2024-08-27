use spin_expressions::Provider;

/// The runtime configuration for the variables factor.
#[derive(Default)]
pub struct RuntimeConfig {
    pub providers: Vec<Box<dyn Provider>>,
}

impl IntoIterator for RuntimeConfig {
    type Item = Box<dyn Provider>;
    type IntoIter = std::vec::IntoIter<Box<dyn Provider>>;

    fn into_iter(self) -> Self::IntoIter {
        self.providers.into_iter()
    }
}
