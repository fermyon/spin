use spin_app::DynamicHostComponent;
use spin_core::HostComponent;

use crate::{LlmDispatch, LlmEngine, AI_MODELS_KEY};

pub struct LlmComponent {
    create_engine: Box<dyn Fn() -> Box<dyn LlmEngine> + Send + Sync>,
}

impl LlmComponent {
    pub fn new<F>(create_engine: F) -> Self
    where
        F: Fn() -> Box<dyn LlmEngine> + Send + Sync + 'static,
    {
        Self {
            create_engine: Box::new(create_engine),
        }
    }
}

impl HostComponent for LlmComponent {
    type Data = LlmDispatch;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        spin_world::v2::llm::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        LlmDispatch {
            engine: (self.create_engine)(),
            allowed_models: Default::default(),
        }
    }
}

impl DynamicHostComponent for LlmComponent {
    fn update_data(
        &self,
        data: &mut Self::Data,
        component: &spin_app::AppComponent,
    ) -> anyhow::Result<()> {
        data.allowed_models = component.get_metadata(AI_MODELS_KEY)?.unwrap_or_default();
        Ok(())
    }
}
