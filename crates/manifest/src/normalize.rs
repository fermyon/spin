//! Manifest normalization functions.

use std::collections::HashSet;

use anyhow::Context;
use spin_common::ui::quoted_path;

use crate::schema::v2::{AppManifest, ComponentSpec, KebabId};

/// Normalizes some optional [`AppManifest`] features into a canonical form:
/// - Inline components in trigger configs are moved into top-level
///   components and replaced with a reference.
/// - Any triggers without an ID are assigned a generated ID.
pub fn normalize_manifest(
    manifest: &mut AppManifest,
    app_root: &std::path::Path,
) -> anyhow::Result<()> {
    // Order is important here!
    normalize_trigger_ids(manifest);
    normalize_external_references(manifest, app_root)?;
    normalize_inline_components(manifest);
    Ok(())
}

fn normalize_external_references(
    manifest: &mut AppManifest,
    app_root: &std::path::Path,
) -> anyhow::Result<()> {
    for trigger in manifest.triggers.values_mut().flatten() {
        let component_specs = trigger
            .component
            .iter_mut()
            .chain(
                trigger
                    .components
                    .values_mut()
                    .flat_map(|specs| specs.0.iter_mut()),
            )
            .collect::<Vec<_>>();

        for spec in component_specs {
            let ComponentSpec::External(path) = spec else {
                continue;
            };

            let abs_path = app_root.join(&path);
            let (abs_path, containing_dir) = if abs_path.is_file() {
                (abs_path, path.parent().unwrap().to_owned())
            } else if abs_path.is_dir() {
                let inferred = abs_path.join("spin-component.toml");
                if inferred.is_file() {
                    (inferred, path.to_owned())
                } else {
                    anyhow::bail!(
                        "{} does not contain a spin-component.toml file",
                        quoted_path(&abs_path)
                    );
                }
            } else {
                anyhow::bail!("{} does not exist", quoted_path(abs_path));
            };

            let toml_text = std::fs::read_to_string(&abs_path)?;
            let mut component: crate::schema::v2::Component = toml::from_str(&toml_text)
                .with_context(|| {
                    format!(
                        "{} is not a valid component manifest",
                        quoted_path(&abs_path)
                    )
                })?;

            relativise_paths(&mut component, &containing_dir);

            // Replace the external component with an inline, which will be normalised
            // in the next pass.
            _ = std::mem::replace(spec, ComponentSpec::Inline(Box::new(component)));
        }
    }

    Ok(())
}

fn relativise_paths(component: &mut crate::schema::v2::Component, relative_to: &std::path::Path) {
    // Empty string means a relative path with no directory component
    if relative_to == std::path::PathBuf::from("") {
        return;
    }

    if let crate::schema::common::ComponentSource::Local(path) = &component.source {
        let adjusted_path = relative_to.join(path);
        component.source = crate::schema::common::ComponentSource::Local(
            adjusted_path.to_string_lossy().to_string(),
        );
    }

    component
        .files
        .iter_mut()
        .for_each(|f| relativise_mount(f, relative_to));
    component
        .exclude_files
        .iter_mut()
        .for_each(|f| *f = relative_to.join(&f).to_string_lossy().to_string());

    if let Some(build) = &mut component.build {
        let workdir = match &build.workdir {
            Some(w) => relative_to.join(w).to_string_lossy().to_string(),
            None => relative_to.to_string_lossy().to_string(),
        };
        build.workdir = Some(workdir);
    }

    // We can't do anything about `tool` entries.  Idea is to inject a `spin:meta` pseudo-tool that
    // consumers of this section can look at to detect that the component they are looking at originated
    // in a subdir.
    let mut tool_meta_map = toml::map::Map::new();
    tool_meta_map.insert(
        "component-manifest-path-base".to_owned(),
        toml::Value::String(relative_to.to_string_lossy().to_string()),
    );
    component.tool.insert("spin:meta".to_owned(), tool_meta_map);
}

fn relativise_mount(mount: &mut crate::schema::v2::WasiFilesMount, relative_to: &std::path::Path) {
    let adjusted_mount = match mount {
        crate::schema::common::WasiFilesMount::Pattern(f) => {
            crate::schema::common::WasiFilesMount::Pattern(
                relative_to.join(f).to_string_lossy().to_string(),
            )
        }
        crate::schema::common::WasiFilesMount::Placement {
            source,
            destination,
        } => crate::schema::common::WasiFilesMount::Placement {
            source: relative_to.join(source).to_string_lossy().to_string(),
            destination: destination.to_string(),
        },
    };
    *mount = adjusted_mount;
}

fn normalize_inline_components(manifest: &mut AppManifest) {
    // Normalize inline components
    let components = &mut manifest.components;

    for trigger in manifest.triggers.values_mut().flatten() {
        let trigger_id = &trigger.id;

        let component_specs = trigger
            .component
            .iter_mut()
            .chain(
                trigger
                    .components
                    .values_mut()
                    .flat_map(|specs| specs.0.iter_mut()),
            )
            .collect::<Vec<_>>();
        let multiple_components = component_specs.len() > 1;

        let mut counter = 1;
        for spec in component_specs {
            if !matches!(spec, ComponentSpec::Inline(_)) {
                continue;
            };

            let inline_id = {
                // Try a "natural" component ID...
                let mut id = KebabId::try_from(format!("{trigger_id}-component"));
                // ...falling back to a counter-based component ID
                if multiple_components
                    || id.is_err()
                    || components.contains_key(id.as_ref().unwrap())
                {
                    id = Ok(loop {
                        let id = KebabId::try_from(format!("inline-component{counter}")).unwrap();
                        if !components.contains_key(&id) {
                            break id;
                        }
                        counter += 1;
                    });
                }
                id.unwrap()
            };

            // Replace the inline component with a reference...
            let inline_spec = std::mem::replace(spec, ComponentSpec::Reference(inline_id.clone()));
            let ComponentSpec::Inline(component) = inline_spec else {
                unreachable!();
            };
            // ...moving the inline component into the top-level components map.
            components.insert(inline_id.clone(), *component);
        }
    }
}

fn normalize_trigger_ids(manifest: &mut AppManifest) {
    let mut trigger_ids = manifest
        .triggers
        .values()
        .flatten()
        .cloned()
        .map(|t| t.id)
        .collect::<HashSet<_>>();
    for (trigger_type, triggers) in &mut manifest.triggers {
        let mut counter = 1;
        for trigger in triggers {
            if !trigger.id.is_empty() {
                continue;
            }
            // Try to assign a "natural" ID to this trigger
            if let Some(ComponentSpec::Reference(component_id)) = &trigger.component {
                let candidate_id = format!("{component_id}-{trigger_type}-trigger");
                if !trigger_ids.contains(&candidate_id) {
                    trigger.id = candidate_id.clone();
                    trigger_ids.insert(candidate_id);
                    continue;
                }
            }
            // Fall back to assigning a counter-based trigger ID
            trigger.id = loop {
                let id = format!("{trigger_type}-trigger{counter}");
                if !trigger_ids.contains(&id) {
                    trigger_ids.insert(id.clone());
                    break id;
                }
                counter += 1;
            }
        }
    }
}
