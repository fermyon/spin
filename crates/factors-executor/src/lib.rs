use std::collections::HashMap;

use anyhow::Context;
use spin_app::{App, AppComponent};
use spin_core::Component;
use spin_factors::{AsInstanceState, ConfiguredApp, RuntimeFactors, RuntimeFactorsInstanceState};

/// A FactorsExecutor manages execution of a Spin app.
///
/// `Factors` is the executor's [`RuntimeFactors`]. `ExecutorInstanceState`
/// holds any other per-instance state needed by the caller.
pub struct FactorsExecutor<T: RuntimeFactors, U = ()> {
    factors: T,
    core_engine: spin_core::Engine<InstanceState<T::InstanceState, U>>,
    configured_app: ConfiguredApp<T>,
    // Maps component IDs -> InstancePres
    component_instance_pres: HashMap<String, InstancePre<T, U>>,
}

type InstancePre<T, U> =
    spin_core::InstancePre<InstanceState<<T as RuntimeFactors>::InstanceState, U>>;

impl<T: RuntimeFactors, U: Send + 'static> FactorsExecutor<T, U> {
    /// Constructs a new executor.
    pub fn new(
        core_config: &spin_core::Config,
        mut factors: T,
        app: App,
        mut component_loader: impl ComponentLoader,
        runtime_config: T::RuntimeConfig,
    ) -> anyhow::Result<Self> {
        let core_engine = {
            let mut builder =
                spin_core::Engine::builder(core_config).context("failed to initialize engine")?;
            factors
                .init(builder.linker())
                .context("failed to initialize factors")?;
            builder.build()
        };

        let configured_app = factors
            .configure_app(app, runtime_config)
            .context("failed to configure app")?;

        let component_instance_pres = configured_app
            .app()
            .components()
            .map(|app_component| {
                let component =
                    component_loader.load_component(core_engine.as_ref(), &app_component)?;
                let instance_pre = core_engine.instantiate_pre(&component)?;
                Ok((app_component.id().to_string(), instance_pre))
            })
            .collect::<anyhow::Result<HashMap<_, _>>>()?;

        Ok(Self {
            factors,
            core_engine,
            configured_app,
            component_instance_pres,
        })
    }

    /// Returns an instance builder for the given component ID.
    pub fn prepare(&mut self, component_id: &str) -> anyhow::Result<FactorsInstanceBuilder<T, U>> {
        let app_component = self
            .configured_app
            .app()
            .get_component(component_id)
            .with_context(|| format!("no such component {component_id:?}"))?;
        let instance_pre = self.component_instance_pres.get(component_id).unwrap();
        let factor_builders = self.factors.prepare(&self.configured_app, component_id)?;
        let store_builder = self.core_engine.store_builder();
        Ok(FactorsInstanceBuilder {
            store_builder,
            factor_builders,
            instance_pre,
            app_component,
            factors: &self.factors,
        })
    }
}

/// A ComponentLoader is responsible for loading Wasmtime [`Component`]s.
pub trait ComponentLoader {
    /// Loads a [`Component`] for the given [`AppComponent`].
    fn load_component(
        &mut self,
        engine: &spin_core::wasmtime::Engine,
        component: &AppComponent,
    ) -> anyhow::Result<Component>;
}

/// A FactorsInstanceBuilder manages the instantiation of a Spin component
/// instance.
pub struct FactorsInstanceBuilder<'a, T: RuntimeFactors, U> {
    app_component: AppComponent<'a>,
    store_builder: spin_core::StoreBuilder,
    factor_builders: T::InstanceBuilders,
    instance_pre: &'a InstancePre<T, U>,
    factors: &'a T,
}

impl<'a, T: RuntimeFactors, U: Send> FactorsInstanceBuilder<'a, T, U> {
    /// Returns the app component for the instance.
    pub fn app_component(&self) -> &AppComponent {
        &self.app_component
    }

    /// Returns the store builder for the instance.
    pub fn store_builder(&mut self) -> &mut spin_core::StoreBuilder {
        &mut self.store_builder
    }

    /// Returns the factor instance builders for the instance.
    pub fn factor_builders(&mut self) -> &mut T::InstanceBuilders {
        &mut self.factor_builders
    }

    /// Instantiates the instance with the given executor instance state
    pub async fn instantiate(
        self,
        executor_instance_state: U,
    ) -> anyhow::Result<(
        spin_core::Instance,
        spin_core::Store<InstanceState<T::InstanceState, U>>,
    )> {
        let instance_state = InstanceState {
            core: Default::default(),
            factors: self.factors.build_instance_state(self.factor_builders)?,
            executor: executor_instance_state,
        };
        let mut store = self.store_builder.build(instance_state)?;
        let instance = self.instance_pre.instantiate_async(&mut store).await?;
        Ok((instance, store))
    }
}

/// InstanceState is the [`spin_core::Store`] `data` for an instance.
pub struct InstanceState<FactorsState, ExecutorInstanceState> {
    core: spin_core::State,
    factors: FactorsState,
    executor: ExecutorInstanceState,
}

impl<T, U> InstanceState<T, U> {
    /// Provides access to the `ExecutorInstanceState`.
    pub fn executor_instance_state(&mut self) -> &mut U {
        &mut self.executor
    }
}

impl<T, U> spin_core::AsState for InstanceState<T, U> {
    fn as_state(&mut self) -> &mut spin_core::State {
        &mut self.core
    }
}

impl<T: RuntimeFactorsInstanceState, U> AsInstanceState<T> for InstanceState<T, U> {
    fn as_instance_state(&mut self) -> &mut T {
        &mut self.factors
    }
}

#[cfg(test)]
mod tests {
    use spin_factor_wasi::{DummyFilesMounter, WasiFactor};
    use spin_factors::RuntimeFactors;
    use spin_factors_test::TestEnvironment;

    use super::*;

    #[derive(RuntimeFactors)]
    struct TestFactors {
        wasi: WasiFactor,
    }

    #[tokio::test]
    async fn instance_builder_works() -> anyhow::Result<()> {
        let factors = TestFactors {
            wasi: WasiFactor::new(DummyFilesMounter),
        };
        let env = TestEnvironment::new(factors);
        let locked = env.build_locked_app().await?;
        let app = App::new("test-app", locked);

        let mut executor = FactorsExecutor::new(
            &Default::default(),
            env.factors,
            app,
            DummyComponentLoader,
            Default::default(),
        )?;

        let mut instance_builder = executor.prepare("empty")?;

        assert_eq!(instance_builder.app_component().id(), "empty");

        instance_builder.store_builder().max_memory_size(1_000_000);

        instance_builder
            .factor_builders()
            .wasi
            .as_mut()
            .unwrap()
            .args(["foo"]);

        let (_instance, _store) = instance_builder.instantiate(()).await?;
        Ok(())
    }

    struct DummyComponentLoader;

    impl ComponentLoader for DummyComponentLoader {
        fn load_component(
            &mut self,
            engine: &spin_core::wasmtime::Engine,
            _component: &AppComponent,
        ) -> anyhow::Result<Component> {
            Component::new(engine, "(component)")
        }
    }
}
