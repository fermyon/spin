use std::path::PathBuf;

use anyhow::{ensure, Context, Result};
use async_trait::async_trait;
use spin_app::{
    locked::{LockedApp, LockedComponentSource},
    AppComponent, Loader,
};
use spin_core::StoreBuilder;
use tokio::fs;

use spin_common::{ui::quoted_path, url::parse_file_url};

/// Compilation status of all components of a Spin application
pub enum CompilationStatus {
    #[cfg(feature = "unsafe-aot-compilation")]
    /// All components are componentized and ahead of time (AOT) compiled to cwasm.
    AllAotComponents,
    /// No components are ahead of time (AOT) compiled.
    NoneAot,
}

/// Loader for the Spin runtime
pub struct TriggerLoader {
    /// Working directory where files for mounting exist.
    working_dir: PathBuf,
    /// Set the static assets of the components in the temporary directory as writable.
    allow_transient_write: bool,
    /// Declares the compilation status of all components of a Spin application.
    compilation_status: CompilationStatus,
}

impl TriggerLoader {
    pub fn new(working_dir: impl Into<PathBuf>, allow_transient_write: bool) -> Self {
        Self {
            working_dir: working_dir.into(),
            allow_transient_write,
            compilation_status: CompilationStatus::NoneAot,
        }
    }

    /// Updates the TriggerLoader to load AOT precompiled components
    ///
    /// **Warning: This feature may bypass important security guarantees of the
    /// Wasmtime
    // security sandbox if used incorrectly! Read this documentation
    // carefully.**
    ///
    /// Usually, components are compiled just-in-time from portable Wasm
    /// sources. This method causes components to instead be loaded
    /// ahead-of-time as Wasmtime-precompiled native executable binaries.
    /// Precompiled binaries must be produced with a compatible Wasmtime engine
    /// using the same Wasmtime version and compiler target settings - typically
    /// by a host with the same processor that will be executing them. See the
    /// Wasmtime documentation for more information:
    /// https://docs.rs/wasmtime/latest/wasmtime/struct.Module.html#method.deserialize
    ///
    /// # Safety
    ///
    /// This method is marked as `unsafe` because it enables potentially unsafe
    /// behavior if
    // used to load malformed or malicious precompiled binaries. Loading sources
    // from an
    /// incompatible Wasmtime engine will fail but is otherwise safe. This
    /// method is safe if it can be guaranteed that `<TriggerLoader as
    /// Loader>::load_component` will only ever be called with a trusted
    /// `LockedComponentSource`. **Precompiled binaries must never be loaded
    /// from untrusted sources.**
    #[cfg(feature = "unsafe-aot-compilation")]
    pub unsafe fn enable_loading_aot_compiled_components(&mut self) {
        self.compilation_status = CompilationStatus::AllAotComponents;
    }
}

#[async_trait]
impl Loader for TriggerLoader {
    async fn load_app(&self, url: &str) -> Result<LockedApp> {
        let path = parse_file_url(url)?;
        let contents = std::fs::read(&path)
            .with_context(|| format!("failed to read manifest at {}", quoted_path(&path)))?;
        let app =
            serde_json::from_slice(&contents).context("failed to parse app lock file JSON")?;
        Ok(app)
    }

    async fn load_component(
        &self,
        engine: &spin_core::wasmtime::Engine,
        source: &LockedComponentSource,
    ) -> Result<spin_core::Component> {
        let source = source
            .content
            .source
            .as_ref()
            .context("LockedComponentSource missing source field")?;
        let path = parse_file_url(source)?;
        match self.compilation_status {
            #[cfg(feature = "unsafe-aot-compilation")]
            CompilationStatus::AllAotComponents => {
                match engine.detect_precompiled_file(&path)?{
                    Some(wasmtime::Precompiled::Component) => {
                        unsafe {
                            spin_core::Component::deserialize_file(engine, &path)
                                .with_context(|| format!("deserializing module {}", quoted_path(&path)))
                        }
                    },
                    Some(wasmtime::Precompiled::Module) => anyhow::bail!("Spin loader is configured to load only AOT compiled components but an AOT compiled module provided at {}", quoted_path(&path)),
                    None => {
                        anyhow::bail!("Spin loader is configured to load only AOT compiled components, but {} is not precompiled", quoted_path(&path))
                    }
                }
            },
            CompilationStatus::NoneAot => {
                let bytes = fs::read(&path).await.with_context(|| {
                format!(
                    "failed to read component source from disk at path {}",
                    quoted_path(&path)
                )
            })?;
            let component = spin_componentize::componentize_if_necessary(&bytes)?;
            spin_core::Component::new(engine, component.as_ref())
                .with_context(|| format!("loading module {}", quoted_path(&path)))
            }
        }
    }

    async fn load_module(
        &self,
        engine: &spin_core::wasmtime::Engine,
        source: &LockedComponentSource,
    ) -> Result<spin_core::Module> {
        let source = source
            .content
            .source
            .as_ref()
            .context("LockedComponentSource missing source field")?;
        let path = parse_file_url(source)?;
        spin_core::Module::from_file(engine, &path)
            .with_context(|| format!("loading module {}", quoted_path(&path)))
    }

    async fn mount_files(
        &self,
        store_builder: &mut StoreBuilder,
        component: &AppComponent,
    ) -> Result<()> {
        for content_dir in component.files() {
            let source_uri = content_dir
                .content
                .source
                .as_deref()
                .with_context(|| format!("Missing 'source' on files mount {content_dir:?}"))?;
            let source_path = self.working_dir.join(parse_file_url(source_uri)?);
            ensure!(
                source_path.is_dir(),
                "TriggerLoader only supports directory mounts; {} is not a directory",
                quoted_path(&source_path),
            );
            let guest_path = content_dir.path.clone();
            if self.allow_transient_write {
                store_builder.read_write_preopened_dir(source_path, guest_path)?;
            } else {
                store_builder.read_only_preopened_dir(source_path, guest_path)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spin_app::locked::ContentRef;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn precompiled_component(file: &mut NamedTempFile) -> LockedComponentSource {
        let wasmtime_engine = wasmtime::Engine::default();
        let component = wasmtime::component::Component::new(&wasmtime_engine, "(component)")
            .unwrap()
            .serialize()
            .unwrap();
        let file_uri = format!("file://{}", file.path().to_str().unwrap());
        file.write_all(&component).unwrap();
        LockedComponentSource {
            content: ContentRef {
                source: Some(file_uri),
                ..Default::default()
            },
            content_type: "application/wasm".to_string(),
        }
    }

    #[cfg(feature = "unsafe-aot-compilation")]
    #[tokio::test]
    async fn load_component_succeeds_for_precompiled_component() {
        let mut file = NamedTempFile::new().unwrap();
        let source = precompiled_component(&mut file);
        let mut loader = super::TriggerLoader::new("/unreferenced", false);
        unsafe {
            loader.enable_loading_aot_compiled_components();
        }
        loader
            .load_component(&spin_core::wasmtime::Engine::default(), &source)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn load_component_fails_for_precompiled_component() {
        let mut file = NamedTempFile::new().unwrap();
        let source = precompiled_component(&mut file);
        let loader = super::TriggerLoader::new("/unreferenced", false);
        let result = loader
            .load_component(&spin_core::wasmtime::Engine::default(), &source)
            .await;
        assert!(result.is_err());
    }
}
