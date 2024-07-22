use std::path::Path;

use anyhow::anyhow;
use spin_common::ui::quoted_path;

pub(crate) struct ComponentToValidate<'a> {
    id: &'a str,
    source: &'a spin_manifest::schema::v2::ComponentSource,
    dependencies: WrappedComponentDependencies,
}

impl<'a> ComponentToValidate<'a> {
    pub fn id(&self) -> &str {
        self.id
    }

    pub fn source_description(&self) -> String {
        match self.source {
            spin_manifest::schema::v2::ComponentSource::Local(path) => {
                format!("file {}", quoted_path(path))
            }
            spin_manifest::schema::v2::ComponentSource::Remote { url, .. } => format!("URL {url}"),
            spin_manifest::schema::v2::ComponentSource::Registry { package, .. } => {
                format!("package {package}")
            }
        }
    }
}

pub fn component_source<'a>(
    app: &'a spin_manifest::schema::v2::AppManifest,
    trigger: &'a spin_manifest::schema::v2::Trigger,
) -> anyhow::Result<ComponentToValidate<'a>> {
    let component_spec = trigger
        .component
        .as_ref()
        .ok_or_else(|| anyhow!("No component specified for trigger {}", trigger.id))?;
    let (id, source, dependencies) = match component_spec {
        spin_manifest::schema::v2::ComponentSpec::Inline(c) => {
            (trigger.id.as_str(), &c.source, &c.dependencies)
        }
        spin_manifest::schema::v2::ComponentSpec::Reference(r) => {
            let id = r.as_ref();
            let Some(component) = app.components.get(r) else {
                anyhow::bail!(
                    "Component {id} specified for trigger {} does not exist",
                    trigger.id
                );
            };
            (id, &component.source, &component.dependencies)
        }
    };
    Ok(ComponentToValidate {
        id,
        source,
        dependencies: WrappedComponentDependencies::new(dependencies),
    })
}

pub struct ResolutionContext {
    wasm_loader: spin_loader::WasmLoader,
}

impl ResolutionContext {
    pub async fn new(base_dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        let wasm_loader =
            spin_loader::WasmLoader::new(base_dir.as_ref().to_owned(), None, None).await?;
        Ok(Self { wasm_loader })
    }

    pub(crate) fn wasm_loader(&self) -> &spin_loader::WasmLoader {
        &self.wasm_loader
    }
}

pub(crate) struct ComponentSourceLoader<'a> {
    wasm_loader: &'a spin_loader::WasmLoader,
    _phantom: std::marker::PhantomData<&'a usize>,
}

impl<'a> ComponentSourceLoader<'a> {
    pub fn new(wasm_loader: &'a spin_loader::WasmLoader) -> Self {
        Self {
            wasm_loader,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<'a> spin_compose::ComponentSourceLoader for ComponentSourceLoader<'a> {
    type Component = ComponentToValidate<'a>;
    type Dependency = WrappedComponentDependency;
    async fn load_component_source(&self, source: &Self::Component) -> anyhow::Result<Vec<u8>> {
        let path = self
            .wasm_loader
            .load_component_source(source.id(), source.source)
            .await?;
        let bytes = tokio::fs::read(&path).await?;
        let component = spin_componentize::componentize_if_necessary(&bytes)?;
        Ok(component.into())
    }

    async fn load_dependency_source(&self, source: &Self::Dependency) -> anyhow::Result<Vec<u8>> {
        let (path, _) = self
            .wasm_loader
            .load_component_dependency(&source.name, &source.dependency)
            .await?;
        let bytes = tokio::fs::read(&path).await?;
        let component = spin_componentize::componentize_if_necessary(&bytes)?;
        Ok(component.into())
    }
}

// This exists only to thwart the orphan rule
pub(crate) struct WrappedComponentDependency {
    name: spin_serde::DependencyName,
    dependency: spin_manifest::schema::v2::ComponentDependency,
}

// To manage lifetimes around the thwarting of the orphan rule
struct WrappedComponentDependencies {
    dependencies: indexmap::IndexMap<spin_serde::DependencyName, WrappedComponentDependency>,
}

impl WrappedComponentDependencies {
    fn new(deps: &spin_manifest::schema::v2::ComponentDependencies) -> Self {
        let dependencies = deps
            .inner
            .clone()
            .into_iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    WrappedComponentDependency {
                        name: k,
                        dependency: v,
                    },
                )
            })
            .collect();
        Self { dependencies }
    }
}

#[async_trait::async_trait]
impl<'a> spin_compose::ComponentLike for ComponentToValidate<'a> {
    type Dependency = WrappedComponentDependency;

    fn dependencies(
        &self,
    ) -> impl std::iter::ExactSizeIterator<Item = (&spin_serde::DependencyName, &Self::Dependency)>
    {
        self.dependencies.dependencies.iter()
    }

    fn id(&self) -> &str {
        self.id
    }
}

#[async_trait::async_trait]
impl spin_compose::DependencyLike for WrappedComponentDependency {
    fn inherit(&self) -> spin_compose::InheritConfiguration {
        // We don't care because this never runs - it is only used to
        // verify import satisfaction. Choosing All avoids the compose
        // algorithm meddling with it using the deny adapter.
        spin_compose::InheritConfiguration::All
    }

    fn export(&self) -> &Option<String> {
        match &self.dependency {
            spin_manifest::schema::v2::ComponentDependency::Version(_) => &None,
            spin_manifest::schema::v2::ComponentDependency::Package { export, .. } => export,
            spin_manifest::schema::v2::ComponentDependency::Local { export, .. } => export,
            spin_manifest::schema::v2::ComponentDependency::HTTP { export, .. } => export,
        }
    }
}
