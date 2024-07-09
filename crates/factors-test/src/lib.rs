use spin_app::locked::LockedApp;
use spin_factors::{
    anyhow::{self, Context},
    serde::de::DeserializeOwned,
    wasmtime::{Config, Engine},
    App, Linker, RuntimeConfigSource, RuntimeFactors,
};
use spin_loader::FilesMountStrategy;

pub use toml::toml;

#[derive(Default)]
pub struct TestEnvironment {
    pub manifest: toml::Table,
    pub runtime_config: toml::Table,
}

impl TestEnvironment {
    /// Starting from a new _uninitialized_ [`RuntimeFactors`], run through the
    /// [`Factor`]s' lifecycle(s) to build a [`RuntimeFactors::InstanceState`].
    pub async fn build_instance_state<T: RuntimeFactors>(
        &mut self,
        mut factors: T,
    ) -> anyhow::Result<T::InstanceState> {
        let mut linker = Self::new_linker::<T>();
        factors.init(&mut linker)?;

        let locked_app = self.build_locked_app().await?;
        let app = App::inert(locked_app);
        let runtime_config = TomlRuntimeConfig(&self.runtime_config);
        let configured_app = factors.configure_app(app, runtime_config)?;

        let component = configured_app
            .app()
            .components()
            .next()
            .context("no components")?;
        factors.build_store_data(&configured_app, component.id())
    }

    pub fn new_linker<T: RuntimeFactors>() -> Linker<T> {
        let engine = Engine::new(Config::new().async_support(true)).expect("engine");
        Linker::<T>::new(&engine)
    }

    pub async fn build_locked_app(&self) -> anyhow::Result<LockedApp> {
        let toml_str = toml::to_string(&self.manifest).context("failed serializing manifest")?;
        let dir = tempfile::tempdir().context("failed creating tempdir")?;
        let path = dir.path().join("spin.toml");
        std::fs::write(&path, toml_str).context("failed writing manifest")?;
        spin_loader::from_file(&path, FilesMountStrategy::Direct, None).await
    }
}

pub struct TomlRuntimeConfig<'a>(&'a toml::Table);

impl RuntimeConfigSource for TomlRuntimeConfig<'_> {
    fn factor_config_keys(&self) -> impl IntoIterator<Item = &str> {
        self.0.keys().map(|key| key.as_str())
    }

    fn get_factor_config<T: DeserializeOwned>(&self, key: &str) -> anyhow::Result<Option<T>> {
        let Some(val) = self.0.get(key) else {
            return Ok(None);
        };
        let config = val.clone().try_into()?;
        Ok(Some(config))
    }
}
