use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use anyhow::{anyhow, Context};
use indexmap::IndexMap;
use regex::Regex;

use crate::{
    constraints::StringConstraints,
    reader::{
        RawExtraOutput, RawParameter, RawTemplateManifest, RawTemplateManifestV1,
        RawTemplateVariant,
    },
    run::{Run, RunOptions},
    store::TemplateLayout,
};

/// A Spin template.
#[derive(Debug)]
pub struct Template {
    id: String,
    tags: HashSet<String>,
    description: Option<String>,
    installed_from: InstalledFrom,
    trigger: TemplateTriggerCompatibility,
    variants: HashMap<TemplateVariantKind, TemplateVariant>,
    parameters: Vec<TemplateParameter>,
    extra_outputs: Vec<ExtraOutputAction>,
    snippets_dir: Option<PathBuf>,
    content_dir: Option<PathBuf>, // TODO: maybe always need a spin.toml file in there?
}

#[derive(Debug)]
enum InstalledFrom {
    Git(String),
    Directory(String),
    Unknown,
}

#[derive(Debug, Eq, PartialEq, Hash)]
enum TemplateVariantKind {
    NewApplication,
    AddComponent,
}

/// The variant mode in which a template should be run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemplateVariantInfo {
    /// Create a new application from the template.
    NewApplication,
    /// Create a new component in an existing application from the template.
    AddComponent {
        /// The manifest to which the component will be added.
        manifest_path: PathBuf,
    },
}

impl TemplateVariantInfo {
    fn kind(&self) -> TemplateVariantKind {
        match self {
            Self::NewApplication => TemplateVariantKind::NewApplication,
            Self::AddComponent { .. } => TemplateVariantKind::AddComponent,
        }
    }

    /// A human-readable description of the variant.
    pub fn description(&self) -> &'static str {
        match self {
            Self::NewApplication => "new application",
            Self::AddComponent { .. } => "add component",
        }
    }

    /// The noun that should be used for the variant in a prompt
    pub fn prompt_noun(&self) -> &'static str {
        match self {
            Self::NewApplication => "application",
            Self::AddComponent { .. } => "component",
        }
    }

    /// The noun that should be used for the variant in a prompt,
    /// qualified with the appropriate a/an article for English
    pub fn articled_noun(&self) -> &'static str {
        match self {
            Self::NewApplication => "an application",
            Self::AddComponent { .. } => "a component",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TemplateVariant {
    skip_files: Vec<String>,
    skip_parameters: Vec<String>,
    snippets: HashMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) enum TemplateTriggerCompatibility {
    Any,
    Only(String),
}

#[derive(Clone, Debug)]
pub(crate) enum TemplateParameterDataType {
    String(StringConstraints),
}

#[derive(Debug)]
pub(crate) struct TemplateParameter {
    id: String,
    data_type: TemplateParameterDataType, // TODO: possibly abstract to a ValidationCriteria type?
    prompt: String,
    default_value: Option<String>,
}

pub(crate) enum ExtraOutputAction {
    CreateDirectory(
        String,
        std::sync::Arc<liquid::Template>,
        crate::reader::CreateLocation,
    ),
}

impl std::fmt::Debug for ExtraOutputAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateDirectory(orig, ..) => {
                f.debug_tuple("CreateDirectory").field(orig).finish()
            }
        }
    }
}

impl Template {
    pub(crate) fn load_from(layout: &TemplateLayout) -> anyhow::Result<Self> {
        let manifest_path = layout.manifest_path();

        let manifest_text = std::fs::read_to_string(&manifest_path).with_context(|| {
            format!(
                "Failed to read template manifest file {}",
                manifest_path.display()
            )
        })?;
        let raw = crate::reader::parse_manifest_toml(manifest_text).with_context(|| {
            format!(
                "Manifest file {} is not a valid manifest",
                manifest_path.display()
            )
        })?;

        validate_manifest(&raw)?;

        let content_dir = if layout.content_dir().exists() {
            Some(layout.content_dir())
        } else {
            None
        };

        let snippets_dir = if layout.snippets_dir().exists() {
            Some(layout.snippets_dir())
        } else {
            None
        };

        let installed_from = read_install_record(layout);

        let template = match raw {
            RawTemplateManifest::V1(raw) => Self {
                id: raw.id.clone(),
                tags: raw.tags.map(Self::normalize_tags).unwrap_or_default(),
                description: raw.description.clone(),
                installed_from,
                trigger: Self::parse_trigger_type(raw.trigger_type, layout),
                variants: Self::parse_template_variants(raw.new_application, raw.add_component),
                parameters: Self::parse_parameters(&raw.parameters)?,
                extra_outputs: Self::parse_extra_outputs(&raw.outputs)?,
                snippets_dir,
                content_dir,
            },
        };
        Ok(template)
    }

    /// The ID of the template. This is used to identify the template
    /// on the Spin command line.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns true if the templates matches the provided set of tags.
    pub fn matches_all_tags(&self, match_set: &[String]) -> bool {
        match_set
            .iter()
            .all(|tag| self.tags().contains(&tag.to_lowercase()))
    }

    /// The set of tags associated with the template, provided by the
    /// template author.
    pub fn tags(&self) -> &HashSet<String> {
        &self.tags
    }

    /// A human-readable description of the template, provided by the
    /// template author.
    pub fn description(&self) -> &Option<String> {
        &self.description
    }

    /// A human-readable description of the template, provided by the
    /// template author, or an empty string if no description was
    /// provided.
    pub fn description_or_empty(&self) -> &str {
        match &self.description {
            Some(s) => s,
            None => "",
        }
    }

    /// The Git repository from which the template was installed, if
    /// it was installed from Git; otherwise None.
    pub fn source_repo(&self) -> Option<&str> {
        // TODO: this is kind of specialised - should we do the discarding of
        // non-Git sources at the application layer?
        match &self.installed_from {
            InstalledFrom::Git(url) => Some(url),
            _ => None,
        }
    }

    /// A human-readable description of where the template was installed
    /// from.
    pub fn installed_from_or_empty(&self) -> &str {
        match &self.installed_from {
            InstalledFrom::Git(repo) => repo,
            InstalledFrom::Directory(path) => path,
            InstalledFrom::Unknown => "",
        }
    }

    fn variant(&self, variant_info: &TemplateVariantInfo) -> Option<&TemplateVariant> {
        let kind = variant_info.kind();
        self.variants.get(&kind)
    }

    pub(crate) fn parameters(
        &self,
        variant_kind: &TemplateVariantInfo,
    ) -> impl Iterator<Item = &TemplateParameter> {
        let variant = self.variant(variant_kind).unwrap(); // TODO: for now
        self.parameters
            .iter()
            .filter(|p| !variant.skip_parameter(p))
    }

    pub(crate) fn parameter(&self, name: impl AsRef<str>) -> Option<&TemplateParameter> {
        self.parameters.iter().find(|p| p.id == name.as_ref())
    }

    pub(crate) fn extra_outputs(&self) -> &[ExtraOutputAction] {
        &self.extra_outputs
    }

    pub(crate) fn content_dir(&self) -> &Option<PathBuf> {
        &self.content_dir
    }

    pub(crate) fn snippets_dir(&self) -> &Option<PathBuf> {
        &self.snippets_dir
    }

    /// Checks if the template supports the specified variant mode.
    pub fn supports_variant(&self, variant: &TemplateVariantInfo) -> bool {
        self.variants.contains_key(&variant.kind())
    }

    pub(crate) fn snippets(&self, variant_kind: &TemplateVariantInfo) -> &HashMap<String, String> {
        let variant = self.variant(variant_kind).unwrap(); // TODO: for now
        &variant.snippets
    }

    /// Creates a runner for the template, governed by the given options. Call
    /// the relevant associated function of the `Run` to execute the template
    /// as appropriate to your application (e.g. `interactive()` to prompt the user
    /// for values and interact with the user at the console).
    pub fn run(self, options: RunOptions) -> Run {
        Run::new(self, options)
    }

    fn normalize_tags(tags: HashSet<String>) -> HashSet<String> {
        tags.into_iter().map(|tag| tag.to_lowercase()).collect()
    }

    fn parse_trigger_type(
        raw: Option<String>,
        layout: &TemplateLayout,
    ) -> TemplateTriggerCompatibility {
        match raw {
            None => Self::infer_trigger_type(layout),
            Some(t) => TemplateTriggerCompatibility::Only(t),
        }
    }

    fn infer_trigger_type(layout: &TemplateLayout) -> TemplateTriggerCompatibility {
        match crate::app_info::AppInfo::from_layout(layout) {
            Some(Ok(app_info)) => match app_info.trigger_type() {
                None => TemplateTriggerCompatibility::Any,
                Some(t) => TemplateTriggerCompatibility::Only(t.to_owned()),
            },
            _ => TemplateTriggerCompatibility::Any, // Fail forgiving
        }
    }

    fn parse_template_variants(
        new_application: Option<RawTemplateVariant>,
        add_component: Option<RawTemplateVariant>,
    ) -> HashMap<TemplateVariantKind, TemplateVariant> {
        let mut variants = HashMap::default();
        if let Some(vt) = Self::get_variant(new_application, true) {
            variants.insert(TemplateVariantKind::NewApplication, vt);
        }
        if let Some(vt) = Self::get_variant(add_component, false) {
            variants.insert(TemplateVariantKind::AddComponent, vt);
        }
        variants
    }

    fn get_variant(
        raw: Option<RawTemplateVariant>,
        default_supported: bool,
    ) -> Option<TemplateVariant> {
        match raw {
            None => {
                if default_supported {
                    Some(Default::default())
                } else {
                    None
                }
            }
            Some(rv) => {
                if rv.supported.unwrap_or(true) {
                    Some(Self::parse_template_variant(rv))
                } else {
                    None
                }
            }
        }
    }

    fn parse_template_variant(raw: RawTemplateVariant) -> TemplateVariant {
        TemplateVariant {
            skip_files: raw.skip_files.unwrap_or_default(),
            skip_parameters: raw.skip_parameters.unwrap_or_default(),
            snippets: raw.snippets.unwrap_or_default(),
        }
    }

    fn parse_parameters(
        raw: &Option<IndexMap<String, RawParameter>>,
    ) -> anyhow::Result<Vec<TemplateParameter>> {
        match raw {
            None => Ok(vec![]),
            Some(parameters) => parameters
                .iter()
                .map(|(k, v)| TemplateParameter::from_raw(k, v))
                .collect(),
        }
    }

    fn parse_extra_outputs(
        raw: &Option<IndexMap<String, RawExtraOutput>>,
    ) -> anyhow::Result<Vec<ExtraOutputAction>> {
        match raw {
            None => Ok(vec![]),
            Some(parameters) => parameters
                .iter()
                .map(|(k, v)| ExtraOutputAction::from_raw(k, v))
                .collect(),
        }
    }

    pub(crate) fn included_files(
        &self,
        base: &std::path::Path,
        all_files: Vec<PathBuf>,
        variant_kind: &TemplateVariantInfo,
    ) -> Vec<PathBuf> {
        let variant = self.variant(variant_kind).unwrap(); // TODO: for now
        all_files
            .into_iter()
            .filter(|path| !variant.skip_file(base, path))
            .collect()
    }

    pub(crate) fn check_compatible_trigger(&self, app_trigger: Option<&str>) -> anyhow::Result<()> {
        // The application we are merging into might not have a trigger yet, in which case
        // we're good to go.
        let Some(app_trigger) = app_trigger else {
            return Ok(());
        };
        match &self.trigger {
            TemplateTriggerCompatibility::Any => Ok(()),
            TemplateTriggerCompatibility::Only(t) => {
                if app_trigger == t {
                    Ok(())
                } else {
                    Err(anyhow!("Component trigger type '{t}' does not match application trigger type '{app_trigger}'"))
                }
            }
        }
    }

    pub(crate) fn check_compatible_manifest_format(
        &self,
        manifest_format: u32,
    ) -> anyhow::Result<()> {
        let Some(content_dir) = &self.content_dir else {
            return Ok(());
        };
        let manifest_tpl = content_dir.join("spin.toml");
        if !manifest_tpl.is_file() {
            return Ok(());
        }

        // We can't load the manifest template because it's not valid TOML until
        // substituted, so GO BIG or at least GO CRUDE.
        let Ok(manifest_tpl_str) = std::fs::read_to_string(&manifest_tpl) else {
            return Ok(());
        };
        let is_v1_tpl = manifest_tpl_str.contains("spin_manifest_version = \"1\"");
        let is_v2_tpl = manifest_tpl_str.contains("spin_manifest_version = 2");

        // If we have not positively identified a format, err on the side of forgiveness
        let positively_identified = is_v1_tpl ^ is_v2_tpl; // exactly one should be true
        if !positively_identified {
            return Ok(());
        }

        let compatible = (is_v1_tpl && manifest_format == 1) || (is_v2_tpl && manifest_format == 2);

        if compatible {
            Ok(())
        } else {
            Err(anyhow!(
                "This template is for a different version of the Spin manifest"
            ))
        }
    }
}

impl TemplateParameter {
    fn from_raw(id: &str, raw: &RawParameter) -> anyhow::Result<Self> {
        let data_type = TemplateParameterDataType::parse(raw)?;

        Ok(Self {
            id: id.to_owned(),
            data_type,
            prompt: raw.prompt.clone(),
            default_value: raw.default_value.clone(),
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn data_type(&self) -> &TemplateParameterDataType {
        &self.data_type
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn default_value(&self) -> &Option<String> {
        &self.default_value
    }

    pub fn validate_value(&self, value: impl AsRef<str>) -> anyhow::Result<String> {
        self.data_type.validate_value(value.as_ref().to_owned())
    }
}

impl TemplateParameterDataType {
    fn parse(raw: &RawParameter) -> anyhow::Result<Self> {
        match &raw.data_type[..] {
            "string" => Ok(Self::String(parse_string_constraints(raw)?)),
            _ => Err(anyhow!("Unrecognised data type '{}'", raw.data_type)),
        }
    }

    fn validate_value(&self, value: String) -> anyhow::Result<String> {
        match self {
            TemplateParameterDataType::String(constraints) => constraints.validate(value),
        }
    }
}

impl ExtraOutputAction {
    fn from_raw(id: &str, raw: &RawExtraOutput) -> anyhow::Result<Self> {
        Ok(match raw {
            RawExtraOutput::CreateDir(create) => {
                let path_template =
                    liquid::Parser::new().parse(&create.path).with_context(|| {
                        format!("Template error: output {id} is not a valid template")
                    })?;
                Self::CreateDirectory(
                    create.path.clone(),
                    std::sync::Arc::new(path_template),
                    create.at.unwrap_or_default(),
                )
            }
        })
    }
}

impl TemplateVariant {
    pub(crate) fn skip_file(&self, base: &std::path::Path, path: &std::path::Path) -> bool {
        self.skip_files
            .iter()
            .map(|s| base.join(s))
            .any(|f| path == f)
    }

    pub(crate) fn skip_parameter(&self, parameter: &TemplateParameter) -> bool {
        self.skip_parameters.iter().any(|p| &parameter.id == p)
    }
}

fn parse_string_constraints(raw: &RawParameter) -> anyhow::Result<StringConstraints> {
    let regex = raw.pattern.as_ref().map(|re| Regex::new(re)).transpose()?;

    Ok(StringConstraints { regex })
}

fn read_install_record(layout: &TemplateLayout) -> InstalledFrom {
    use crate::reader::{parse_installed_from, RawInstalledFrom};

    let installed_from_text = std::fs::read_to_string(layout.installation_record_file()).ok();
    match installed_from_text.and_then(parse_installed_from) {
        Some(RawInstalledFrom::Git { git }) => InstalledFrom::Git(git),
        Some(RawInstalledFrom::File { dir }) => InstalledFrom::Directory(dir),
        None => InstalledFrom::Unknown,
    }
}

fn validate_manifest(raw: &RawTemplateManifest) -> anyhow::Result<()> {
    match raw {
        RawTemplateManifest::V1(raw) => validate_v1_manifest(raw),
    }
}

fn validate_v1_manifest(raw: &RawTemplateManifestV1) -> anyhow::Result<()> {
    if raw.custom_filters.is_some() {
        anyhow::bail!("Custom filters are not supported in this version of Spin. Please update your template.");
    }
    Ok(())
}
