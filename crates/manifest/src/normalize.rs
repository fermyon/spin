//! Manifest normalization functions.

use std::collections::HashSet;

use crate::schema::v2::{self, AppManifest, ComponentSpec, KebabId};

/// Extracts each `ComponentSpec::Inline` into `AppManifest::component`s,
/// replacing it with a `ComponentSpec::Reference`.
pub fn normalize_inline_components(manifest: &mut AppManifest) {
    let components = &mut manifest.components;
    for trigger in &mut manifest.triggers.values_mut().flatten() {
        let trigger_id = &trigger.id;
        let component_specs = trigger.component.iter_mut().chain(
            trigger
                .components
                .values_mut()
                .flat_map(|specs| specs.0.iter_mut()),
        );

        let mut counter = 1;
        for spec in component_specs {
            if !matches!(spec, ComponentSpec::Inline(_)) {
                continue;
            };

            // Generate an unused component ID
            let inline_id = loop {
                let id =
                    KebabId::try_from(format!("{trigger_id}-inline-component{counter}")).unwrap();
                if !components.contains_key(&id) {
                    break id;
                }
                counter += 1;
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

/// Generates IDs for any [`Trigger`]s without one.
pub fn normalize_trigger_ids(typed_triggers: &mut v2::Map<String, Vec<v2::Trigger>>) {
    let mut trigger_ids = typed_triggers
        .values()
        .flatten()
        .cloned()
        .map(|t| t.id)
        .collect::<HashSet<_>>();
    for (trigger_type, triggers) in typed_triggers {
        let mut counter = 1;
        for trigger in triggers {
            if !trigger.id.is_empty() {
                continue;
            }
            // Generate an unused trigger ID
            trigger.id = loop {
                let id = format!("{trigger_type}-{counter}");
                if !trigger_ids.contains(&id) {
                    trigger_ids.insert(id.clone());
                    break id;
                }
                counter += 1;
            }
        }
    }
}
