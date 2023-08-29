use std::{
    ffi::OsString,
    fmt::Debug,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use clap::{CommandFactory, Parser};
use reqwest::Url;
use spin_app::locked::LockedApp;
use spin_manifest::ApplicationTrigger;
use spin_oci::OciLoader;
use spin_trigger::cli::{SPIN_LOCAL_APP_DIR, SPIN_LOCKED_URL, SPIN_WORKING_DIR};
use tempfile::TempDir;

use crate::opts::*;

const APPLICATION_OPT: &str = "APPLICATION";

/// Start the Fermyon runtime.
#[derive(Parser, Debug, Default)]
#[clap(
    about = "Start the Spin application",
    allow_hyphen_values = true,
    disable_help_flag = true
)]
pub struct UpCommand {
    #[clap(short = 'h', long = "help")]
    pub help: bool,

    /// The application to run. This may be a manifest (spin.toml) file, a
    /// directory containing a spin.toml file, or a remote registry reference.
    /// If omitted, it defaults to "spin.toml".
    #[clap(
        name = APPLICATION_OPT,
        short = 'f',
        long = "from",
        group = "source",
    )]
    pub app_source: Option<String>,

    /// The application to run. This is the same as `--from` but forces the
    /// application to be interpreted as a file or directory path.
    #[clap(
        hide = true,
        name = APP_MANIFEST_FILE_OPT,
        long = "from-file",
        alias = "file",
        group = "source",
    )]
    pub file_source: Option<PathBuf>,

    /// The application to run. This is the same as `--from` but forces the
    /// application to be interpreted as an OCI registry reference.
    #[clap(
        hide = true,
        name = FROM_REGISTRY_OPT,
        long = "from-registry",
        group = "source",
    )]
    pub registry_source: Option<String>,

    /// Ignore server certificate errors from bindle server or registry
    #[clap(
        name = INSECURE_OPT,
        short = 'k',
        long = "insecure",
        takes_value = false,
    )]
    pub insecure: bool,

    /// Pass an environment variable (key=value) to all components of the application.
    #[clap(short = 'e', long = "env", parse(try_from_str = parse_env_var))]
    pub env: Vec<(String, String)>,

    /// Temporary directory for the static assets of the components.
    #[clap(long = "temp")]
    pub tmp: Option<PathBuf>,

    /// For local apps with directory mounts and no excluded files, mount them directly instead of using a temporary
    /// directory.
    ///
    /// This allows you to update the assets on the host filesystem such that the updates are visible to the guest
    /// without a restart.  This cannot be used with bindle apps or apps which use file patterns and/or exclusions.
    #[clap(long, takes_value = false)]
    pub direct_mounts: bool,

    /// All other args, to be passed through to the trigger
    #[clap(hide = true)]
    pub trigger_args: Vec<OsString>,
}

impl UpCommand {
    pub async fn run(self) -> Result<()> {
        // For displaying help, first print `spin up`'s own usage text, then
        // attempt to load an app and print trigger-type-specific usage.
        let help = self.help;
        if help {
            Self::command()
                .name("spin-up")
                .bin_name("spin up")
                .print_help()?;
            println!();
        }
        self.run_inner().await.or_else(|err| {
            if help {
                tracing::warn!("Error resolving trigger-specific help: {}", err);
                Ok(())
            } else {
                Err(err)
            }
        })
    }

    async fn run_inner(self) -> Result<()> {
        let app_source = self.resolve_app_source();

        if app_source == AppSource::None {
            if self.help {
                return self
                    .run_trigger(trigger_command(HELP_ARGS_ONLY_TRIGGER_TYPE), None)
                    .await;
            } else {
                bail!("Default file '{DEFAULT_MANIFEST_FILE}' not found. Run `spin up --from <APPLICATION>`, or `spin up --help` for usage.");
            }
        }

        let working_dir_holder = match &self.tmp {
            None => WorkingDirectory::Temporary(TempDir::with_prefix("spinup-")?),
            Some(d) => WorkingDirectory::Given(d.to_owned()),
        };
        let working_dir = working_dir_holder.path().canonicalize()?;

        let mut locked_app = match &app_source {
            AppSource::None => bail!("Internal error - should have shown help"),
            AppSource::File(path) => self.prepare_app_from_file(path, &working_dir).await?,
            AppSource::OciRegistry(oci) => self.prepare_app_from_oci(oci, &working_dir).await?,
            AppSource::Unresolvable(err) => bail!("{err}"),
        };

        let trigger_cmd = trigger_command_from_locked_app(&locked_app)?;

        if self.help {
            return self.run_trigger(trigger_cmd, None).await;
        }

        self.update_locked_app(&mut locked_app);

        let local_app_dir = app_source.local_app_dir().map(Into::into);

        let run_opts = RunTriggerOpts {
            locked_app,
            working_dir,
            local_app_dir,
        };

        self.run_trigger(trigger_cmd, Some(run_opts)).await
    }

    async fn run_trigger(
        self,
        trigger_cmd: Vec<String>,
        opts: Option<RunTriggerOpts>,
    ) -> Result<(), anyhow::Error> {
        // The docs for `current_exe` warn that this may be insecure because it could be executed
        // via hard-link. I think it should be fine as long as we aren't `setuid`ing this binary.
        let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
        cmd.args(&trigger_cmd);

        if let Some(RunTriggerOpts {
            locked_app,
            working_dir,
            local_app_dir,
        }) = opts
        {
            let locked_url = self.write_locked_app(&locked_app, &working_dir).await?;

            cmd.env(SPIN_LOCKED_URL, locked_url)
                .env(SPIN_WORKING_DIR, &working_dir)
                .args(&self.trigger_args);

            if let Some(local_app_dir) = local_app_dir {
                cmd.env(SPIN_LOCAL_APP_DIR, local_app_dir);
            }
        } else {
            cmd.arg("--help-args-only");
        }

        tracing::trace!("Running trigger executor: {:?}", cmd);

        let mut child = cmd.spawn().context("Failed to execute trigger")?;

        // Terminate trigger executor if `spin up` itself receives a termination signal
        #[cfg(not(windows))]
        {
            // https://github.com/nix-rust/nix/issues/656
            let pid = nix::unistd::Pid::from_raw(child.id() as i32);
            ctrlc::set_handler(move || {
                if let Err(err) = nix::sys::signal::kill(pid, nix::sys::signal::SIGTERM) {
                    tracing::warn!("Failed to kill trigger handler process: {:?}", err)
                }
            })?;
        }

        let status = child.wait()?;
        if status.success() {
            Ok(())
        } else {
            Err(crate::subprocess::ExitStatusError::new(status).into())
        }
    }

    fn resolve_app_source(&self) -> AppSource {
        match (&self.app_source, &self.file_source, &self.registry_source) {
            (None, None, None) => self.default_manifest_or_none(),
            (Some(source), None, None) => Self::infer_source(source),
            (None, Some(file), None) => Self::infer_file_source(file.to_owned()),
            (None, None, Some(reference)) => AppSource::OciRegistry(reference.to_owned()),
            _ => AppSource::unresolvable("More than one application source was specified"),
        }
    }

    fn default_manifest_or_none(&self) -> AppSource {
        let default_manifest = PathBuf::from(DEFAULT_MANIFEST_FILE);
        if default_manifest.exists() {
            AppSource::File(default_manifest)
        } else if self.trigger_args_look_file_like() {
            let msg = format!(
                "Default file 'spin.toml' not found. Did you mean `spin up -f {}`?`",
                self.trigger_args[0].to_string_lossy()
            );
            AppSource::Unresolvable(msg)
        } else {
            AppSource::None
        }
    }

    fn infer_source(source: &str) -> AppSource {
        let path = PathBuf::from(source);
        if path.exists() {
            Self::infer_file_source(path)
        } else if spin_oci::is_probably_oci_reference(source) {
            AppSource::OciRegistry(source.to_owned())
        } else {
            AppSource::Unresolvable(format!("File or directory '{source}' not found. If you meant to load from a registry, use the `--from-registry` option."))
        }
    }

    fn infer_file_source(path: impl Into<PathBuf>) -> AppSource {
        match spin_common::paths::resolve_manifest_file_path(path.into()) {
            Ok(file) => AppSource::File(file),
            Err(e) => AppSource::Unresolvable(e.to_string()),
        }
    }

    fn trigger_args_look_file_like(&self) -> bool {
        // Heuristic for the user typing `spin up foo` instead of `spin up -f foo` - in the
        // first case `foo` gets interpreted as a trigger arg which is probably not what the
        // user intended.
        !self.trigger_args.is_empty() && !self.trigger_args[0].to_string_lossy().starts_with('-')
    }

    async fn write_locked_app(
        &self,
        locked_app: &LockedApp,
        working_dir: &Path,
    ) -> Result<String, anyhow::Error> {
        let locked_path = working_dir.join("spin.lock");
        let locked_app_contents =
            serde_json::to_vec_pretty(&locked_app).context("failed to serialize locked app")?;
        tokio::fs::write(&locked_path, locked_app_contents)
            .await
            .with_context(|| format!("failed to write {:?}", locked_path))?;
        let locked_url = Url::from_file_path(&locked_path)
            .map_err(|_| anyhow!("cannot convert to file URL: {locked_path:?}"))?
            .to_string();

        Ok(locked_url)
    }

    async fn prepare_app_from_file(
        &self,
        manifest_path: &Path,
        working_dir: &Path,
    ) -> Result<LockedApp> {
        let asset_dst = if self.direct_mounts {
            None
        } else {
            Some(working_dir)
        };

        let app = spin_loader::from_file(manifest_path, asset_dst).await?;

        spin_trigger::locked::build_locked_app(app, working_dir)
    }

    async fn prepare_app_from_oci(&self, reference: &str, working_dir: &Path) -> Result<LockedApp> {
        let mut client = spin_oci::Client::new(self.insecure, None)
            .await
            .context("cannot create registry client")?;

        OciLoader::new(working_dir)
            .load_app(&mut client, reference)
            .await
    }

    fn update_locked_app(&self, locked_app: &mut LockedApp) {
        // Apply --env to component environments
        if !self.env.is_empty() {
            for component in locked_app.components.iter_mut() {
                component.env.extend(self.env.iter().cloned());
            }
        }
    }
}

struct RunTriggerOpts {
    locked_app: LockedApp,
    working_dir: PathBuf,
    local_app_dir: Option<PathBuf>,
}

enum WorkingDirectory {
    Given(PathBuf),
    Temporary(TempDir),
}

impl WorkingDirectory {
    fn path(&self) -> &Path {
        match self {
            Self::Given(p) => p,
            Self::Temporary(t) => t.path(),
        }
    }
}

// Parse the environment variables passed in `key=value` pairs.
fn parse_env_var(s: &str) -> Result<(String, String)> {
    let parts: Vec<_> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        bail!("Environment variable must be of the form `key=value`");
    }
    Ok((parts[0].to_owned(), parts[1].to_owned()))
}

fn resolve_trigger_plugin(trigger_type: &str) -> Result<String> {
    use crate::commands::plugins::PluginCompatibility;
    use spin_plugins::manager::PluginManager;

    let subcommand = format!("trigger-{trigger_type}");
    let plugin_manager = PluginManager::try_default()
        .with_context(|| format!("Failed to access plugins looking for '{subcommand}'"))?;
    let plugin_store = plugin_manager.store();
    let is_installed = plugin_store
        .installed_manifests()
        .unwrap_or_default()
        .iter()
        .any(|m| m.name() == subcommand);

    if is_installed {
        return Ok(subcommand);
    }

    if let Some(known) = plugin_store
        .catalogue_manifests()
        .unwrap_or_default()
        .iter()
        .find(|m| m.name() == subcommand)
    {
        match PluginCompatibility::for_current(known) {
            PluginCompatibility::Compatible => Err(anyhow!("No built-in trigger named '{trigger_type}', but plugin '{subcommand}' is available to install")),
            _ => Err(anyhow!("No built-in trigger named '{trigger_type}', and plugin '{subcommand}' is not compatible"))
        }
    } else {
        Err(anyhow!("No built-in trigger named '{trigger_type}', and no plugin named '{subcommand}' was found"))
    }
}

fn trigger_command(trigger_type: &str) -> Vec<String> {
    vec!["trigger".to_owned(), trigger_type.to_owned()]
}

fn trigger_command_from_locked_app(locked_app: &LockedApp) -> Result<Vec<String>> {
    let trigger_metadata = locked_app
        .metadata
        .get("trigger")
        .cloned()
        .ok_or_else(|| anyhow!("missing trigger metadata in locked application"))?;

    let trigger_info: ApplicationTrigger = serde_json::from_value(trigger_metadata)
        .context("deserializing trigger type from locked application")?;

    match trigger_info {
        ApplicationTrigger::Http(_) => Ok(trigger_command("http")),
        ApplicationTrigger::Redis(_) => Ok(trigger_command("redis")),
        ApplicationTrigger::Mqtt(_) => Ok(trigger_command("mqtt")),
        ApplicationTrigger::External(cfg) => {
            resolve_trigger_plugin(cfg.trigger_type()).map(|p| vec![p])
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum AppSource {
    None,
    File(PathBuf),
    OciRegistry(String),
    Unresolvable(String),
}

impl AppSource {
    fn unresolvable(message: impl Into<String>) -> Self {
        Self::Unresolvable(message.into())
    }

    fn local_app_dir(&self) -> Option<&Path> {
        match self {
            Self::File(path) => path.parent().or_else(|| {
                tracing::warn!("Error finding local app dir from source {path:?}");
                None
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn repo_path(path: &str) -> String {
        // This is all strings and format because app_source is a string not a PathBuf
        let repo_base = env!("CARGO_MANIFEST_DIR");
        format!("{repo_base}/{path}")
    }

    #[test]
    fn can_infer_files() {
        let file = repo_path("examples/http-rust/spin.toml");

        let source = UpCommand {
            app_source: Some(file.clone()),
            ..Default::default()
        }
        .resolve_app_source();

        assert_eq!(AppSource::File(PathBuf::from(file)), source);
    }

    #[test]
    fn can_infer_directories() {
        let dir = repo_path("examples/http-rust");

        let source = UpCommand {
            app_source: Some(dir.clone()),
            ..Default::default()
        }
        .resolve_app_source();

        assert_eq!(
            AppSource::File(PathBuf::from(dir).join("spin.toml")),
            source
        );
    }

    #[test]
    fn reject_nonexistent_files() {
        let file = repo_path("src/commands/biscuits.toml");

        let source = UpCommand {
            app_source: Some(file),
            ..Default::default()
        }
        .resolve_app_source();

        assert!(matches!(source, AppSource::Unresolvable(_)));
    }

    #[test]
    fn reject_nonexistent_files_relative_path() {
        let file = "zoink/honk/biscuits.toml".to_owned(); // NOBODY CREATE THIS OKAY

        let source = UpCommand {
            app_source: Some(file),
            ..Default::default()
        }
        .resolve_app_source();

        assert!(matches!(source, AppSource::Unresolvable(_)));
    }

    #[test]
    fn reject_unsuitable_directories() {
        let dir = repo_path("src/commands");

        let source = UpCommand {
            app_source: Some(dir),
            ..Default::default()
        }
        .resolve_app_source();

        assert!(matches!(source, AppSource::Unresolvable(_)));
    }

    #[test]
    fn can_infer_oci_registry_reference() {
        let reference = "ghcr.io/fermyon/noodles:v1".to_owned();

        let source = UpCommand {
            app_source: Some(reference.clone()),
            ..Default::default()
        }
        .resolve_app_source();

        assert_eq!(AppSource::OciRegistry(reference), source);
    }

    #[test]
    fn can_infer_docker_registry_reference() {
        // Testing that the magic docker heuristic doesn't misfire here.
        let reference = "docker.io/fermyon/noodles".to_owned();

        let source = UpCommand {
            app_source: Some(reference.clone()),
            ..Default::default()
        }
        .resolve_app_source();

        assert_eq!(AppSource::OciRegistry(reference), source);
    }

    #[test]
    fn can_reject_complete_gibberish() {
        let garbage = repo_path("ftp://🤡***🤡 HELLO MR CLOWN?!");

        let source = UpCommand {
            app_source: Some(garbage),
            ..Default::default()
        }
        .resolve_app_source();

        // Honestly I feel Unresolvable might be a bit weak sauce for this case
        assert!(matches!(source, AppSource::Unresolvable(_)));
    }

    #[test]
    fn parses_untyped_source() {
        UpCommand::try_parse_from(["up", "-f", "ghcr.io/example/test:v1"])
            .expect("Failed to parse --from with option");
        UpCommand::try_parse_from(["up", "-f", "ghcr.io/example/test:v1", "--direct-mounts"])
            .expect("Failed to parse --from with option");
        UpCommand::try_parse_from([
            "up",
            "-f",
            "ghcr.io/example/test:v1",
            "--listen",
            "127.0.0.1:39453",
        ])
        .expect("Failed to parse --from with trigger option");
    }

    #[test]
    fn parses_typed_source() {
        UpCommand::try_parse_from(["up", "--from-registry", "ghcr.io/example/test:v1"])
            .expect("Failed to parse --from-registry with option");
        UpCommand::try_parse_from([
            "up",
            "--from-registry",
            "ghcr.io/example/test:v1",
            "--direct-mounts",
        ])
        .expect("Failed to parse --from-registry with option");
        UpCommand::try_parse_from([
            "up",
            "--from-registry",
            "ghcr.io/example/test:v1",
            "--listen",
            "127.0.0.1:39453",
        ])
        .expect("Failed to parse --from-registry with trigger option");
    }

    #[test]
    fn parses_implicit_source() {
        UpCommand::try_parse_from(["up"]).expect("Failed to parse implicit source with option");
        UpCommand::try_parse_from(["up", "--direct-mounts"])
            .expect("Failed to parse implicit source with option");
        UpCommand::try_parse_from(["up", "--listen", "127.0.0.1:39453"])
            .expect("Failed to parse implicit source with trigger option");
    }
}
