use spin_app::locked::LockedApp;
use spin_factors::{
    anyhow::{self, Context},
    wasmtime::{component::Linker, Config, Engine},
    App, RuntimeFactors,
};
use spin_loader::FilesMountStrategy;

pub use toml::toml;

/// A test environment for building [`RuntimeFactors`] instances.
pub struct TestEnvironment {
    /// The `spin.toml` manifest.
    pub manifest: toml::Table,
}

impl Default for TestEnvironment {
    fn default() -> Self {
        let manifest = toml! {
            spin_manifest_version = 2

            [application]
            name = "test-app"

            [[trigger.test-trigger]]

            [component.empty]
            source = "does-not-exist.wasm"
        };
        Self { manifest }
    }
}

impl TestEnvironment {
    /// Builds a TestEnvironment by extending a default manifest with the given
    /// manifest TOML.
    ///
    /// The default manifest includes boilerplate like the
    /// `spin_manifest_version` and `[application]` section, so you typically
    /// need to pass only a `[component.test-component]` section.
    pub fn default_manifest_extend(manifest_merge: toml::Table) -> Self {
        let mut env = Self::default();
        env.manifest.extend(manifest_merge);
        env
    }

    /// Starting from a new _uninitialized_ [`RuntimeFactors`], run through the
    /// [`Factor`]s' lifecycle(s) to build a [`RuntimeFactors::InstanceState`]
    /// for the last component defined in the manifest.
    pub async fn build_instance_state<'a, T, C, E>(
        &'a self,
        mut factors: T,
        runtime_config: C,
    ) -> anyhow::Result<T::InstanceState>
    where
        T: RuntimeFactors,
        C: TryInto<T::RuntimeConfig, Error = E>,
        E: Into<anyhow::Error>,
    {
        let mut linker = Self::new_linker::<T::InstanceState>();
        factors.init(&mut linker)?;

        let locked_app = self
            .build_locked_app()
            .await
            .context("failed to build locked app")?;
        let app = App::new("test-app", locked_app);
        let configured_app =
            factors.configure_app(app, runtime_config.try_into().map_err(|e| e.into())?)?;

        let component =
            configured_app.app().components().last().context(
                "expected configured app to have at least one component, but it did not",
            )?;
        let builders = factors.prepare(&configured_app, component.id())?;

        Ok(factors.build_instance_state(builders)?)
    }

    pub fn new_linker<T>() -> Linker<T> {
        let engine = Engine::new(Config::new().async_support(true))
            .expect("wasmtime engine failed to initialize");
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
