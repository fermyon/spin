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
    core_engine: spin_core::Engine<InstanceState<T::InstanceState, U>>,
    factors: T,
    hooks: Vec<Box<dyn ExecutorHooks<T, U>>>,
}

impl<T: RuntimeFactors, U: Send + 'static> FactorsExecutor<T, U> {
    /// Constructs a new executor.
    pub fn new(
        mut core_engine_builder: spin_core::EngineBuilder<
            InstanceState<<T as RuntimeFactors>::InstanceState, U>,
        >,
        mut factors: T,
    ) -> anyhow::Result<Self> {
        factors
            .init(core_engine_builder.linker())
            .context("failed to initialize factors")?;
        Ok(Self {
            factors,
            core_engine: core_engine_builder.build(),
            hooks: Default::default(),
        })
    }

    /// Adds the given [`ExecutorHooks`] to this executor.
    ///
    /// Hooks are run in the order they are added.
    pub fn add_hooks(&mut self, hooks: impl ExecutorHooks<T, U> + 'static) {
        self.hooks.push(Box::new(hooks));
    }

    /// Loads a [`FactorsApp`] with this executor.
    pub fn load_app(
        mut self,
        app: App,
        runtime_config: T::RuntimeConfig,
        mut component_loader: impl ComponentLoader,
    ) -> anyhow::Result<FactorsExecutorApp<T, U>> {
        let configured_app = self
            .factors
            .configure_app(app, runtime_config)
            .context("failed to configure app")?;

        for hooks in &mut self.hooks {
            hooks.configure_app(&configured_app)?;
        }

        let component_instance_pres = configured_app
            .app()
            .components()
            .map(|app_component| {
                let component =
                    component_loader.load_component(self.core_engine.as_ref(), &app_component)?;
                let instance_pre = self.core_engine.instantiate_pre(&component)?;
                Ok((app_component.id().to_string(), instance_pre))
            })
            .collect::<anyhow::Result<HashMap<_, _>>>()?;

        Ok(FactorsExecutorApp {
            executor: self,
            configured_app,
            component_instance_pres,
        })
    }
}

pub trait ExecutorHooks<T: RuntimeFactors, U>: Send + Sync {
    /// Configure app hooks run immediately after [`RuntimeFactors::configure_app`].
    fn configure_app(&mut self, configured_app: &ConfiguredApp<T>) -> anyhow::Result<()> {
        let _ = configured_app;
        Ok(())
    }

    /// Prepare instance hooks run immediately before [`FactorsExecutor::prepare`] returns.
    fn prepare_instance(&self, builder: &mut FactorsInstanceBuilder<T, U>) -> anyhow::Result<()> {
        let _ = builder;
        Ok(())
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

type InstancePre<T, U> =
    spin_core::InstancePre<InstanceState<<T as RuntimeFactors>::InstanceState, U>>;

/// A FactorsExecutorApp represents a loaded Spin app, ready for instantiation.
pub struct FactorsExecutorApp<T: RuntimeFactors, U> {
    executor: FactorsExecutor<T, U>,
    configured_app: ConfiguredApp<T>,
    // Maps component IDs -> InstancePres
    component_instance_pres: HashMap<String, InstancePre<T, U>>,
}

impl<T: RuntimeFactors, U: Send + 'static> FactorsExecutorApp<T, U> {
    pub fn engine(&self) -> &spin_core::Engine<InstanceState<T::InstanceState, U>> {
        &self.executor.core_engine
    }

    pub fn app(&self) -> &App {
        self.configured_app.app()
    }

    pub fn get_component(&self, component_id: &str) -> anyhow::Result<&Component> {
        let instance_pre = self
            .component_instance_pres
            .get(component_id)
            .with_context(|| format!("no such component {component_id:?}"))?;
        Ok(instance_pre.component())
    }

    /// Returns an instance builder for the given component ID.
    pub fn prepare(&self, component_id: &str) -> anyhow::Result<FactorsInstanceBuilder<T, U>> {
        let app_component = self
            .configured_app
            .app()
            .get_component(component_id)
            .with_context(|| format!("no such component {component_id:?}"))?;

        let instance_pre = self.component_instance_pres.get(component_id).unwrap();

        let factor_builders = self
            .executor
            .factors
            .prepare(&self.configured_app, component_id)?;

        let store_builder = self.executor.core_engine.store_builder();

        let mut builder = FactorsInstanceBuilder {
            store_builder,
            factor_builders,
            instance_pre,
            app_component,
            factors: &self.executor.factors,
        };

        for hooks in &self.executor.hooks {
            hooks.prepare_instance(&mut builder)?;
        }

        Ok(builder)
    }
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

impl<'a, T: RuntimeFactors, U> FactorsInstanceBuilder<'a, T, U> {
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
}

impl<'a, T: RuntimeFactors, U: Send> FactorsInstanceBuilder<'a, T, U> {
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
pub struct InstanceState<T, U> {
    core: spin_core::State,
    factors: T,
    executor: U,
}

impl<T, U> InstanceState<T, U> {
    /// Provides access to the [`spin_core::State`].
    pub fn core_state(&self) -> &spin_core::State {
        &self.core
    }

    /// Provides access to the [`RuntimeFactors::InstanceState`].
    pub fn factors_instance_state(&mut self) -> &mut T {
        &mut self.factors
    }

    /// Provides access to the `Self::ExecutorInstanceState`.
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

        let engine_builder = spin_core::Engine::builder(&Default::default())?;
        let executor = FactorsExecutor::new(engine_builder, env.factors)?;

        let factors_app = executor.load_app(app, Default::default(), DummyComponentLoader)?;

        let mut instance_builder = factors_app.prepare("empty")?;

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
