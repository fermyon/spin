use crate::commands::up;
use anyhow::{anyhow, Context, Error, Result};
use bindle::{
    provider::file::FileProvider,
    search::{Search, SearchOptions, StrictEngine},
    Invoice,
};
use clap::Parser;
use futures::{stream, StreamExt, TryStreamExt};
use spin_loader::bindle::config::RawAppManifest;
use spin_manifest::{
    Application, ApplicationInformation, ApplicationOrigin, CoreComponent, ModuleSource,
    SpinVersion, WasmConfig,
};
use spin_trigger::{
    loader::TriggerLoader, HostComponentInitData, RuntimeConfig, TriggerExecutor,
    TriggerExecutorBuilder,
};
use spin_trigger_http::HttpTrigger;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    rc::Rc,
    str,
};
use url::Url;

const MAX_CONCURRENCY: usize = 16;

#[derive(Parser, Debug, Default)]
#[clap(about = "Instantiate all apps in a repository")]
pub struct CraterCommand {
    /// Directory containing bindle repository.
    #[clap(env = "BINDLE_DIRECTORY")]
    pub directory: PathBuf,

    /// Path to file containing newline-delimited list of app IDs indicating which apps to test.
    #[clap(long)]
    pub filter: Option<PathBuf>,

    /// Path to file containing newline-delimited list of app IDs indicating which apps *not* to test.
    #[clap(long)]
    pub exclude: Option<PathBuf>,

    /// Path to file which will be populated with a newline-delimited list of failed app IDs
    #[clap(long)]
    pub failures: Option<PathBuf>,
}

impl CraterCommand {
    /// Iterate over all the apps in the specified Bindle repository, attempt to instantiate all the components of
    /// each app, and print the result of each attempt to stdout.
    pub async fn run(self) -> Result<()> {
        // Note that we access the Bindle repository directly instead of going through a Bindle server since the
        // latter is prohibitively slow (especially for large invoices).  That means duplicating some of the code
        // in `spin-loader`, unfortunately, but it also allows us to skip copying static assets, etc. since they're
        // not relevant.

        let filter = read_id_list(&self.filter).await?;
        let exclude = read_id_list(&self.exclude).await?;
        let directory = match std::fs::canonicalize(self.directory) {
            Ok(d) => Rc::from(d.as_ref()),
            Err(e) => {
                return Err(e).context("could not canonicalize the bindle repository directory")
            }
        };
        let index = StrictEngine::default();

        println!("loading index...");
        FileProvider::new(&directory, index.clone()).await;
        println!("index loaded; getting invoices...");

        let mut invoices = Vec::new();
        {
            let mut offset = 0;
            loop {
                let matches = index
                    .query(
                        "",
                        "",
                        SearchOptions {
                            offset,
                            limit: 100,
                            strict: true,
                            yanked: false,
                        },
                    )
                    .await?;

                if matches.invoices.is_empty() {
                    break;
                } else {
                    offset += u64::try_from(matches.invoices.len()).unwrap();
                    invoices.extend(matches.invoices);
                }
            }
        }

        let total = invoices.len();
        println!("got {total} invoices");
        if let Some(filter) = filter.as_ref() {
            println!("filtering to {} apps", filter.len());
        }

        let mut stream = stream::iter(invoices.into_iter().enumerate().filter(|(_, invoice)| {
            filter
                .as_ref()
                .map(|filter| filter.contains(&invoice.bindle.id.to_string()))
                .unwrap_or(true)
                && exclude
                    .as_ref()
                    .map(|exclude| !exclude.contains(&invoice.bindle.id.to_string()))
                    .unwrap_or(true)
        }))
        .map(|(index, invoice)| {
            let directory = directory.clone();

            async move {
                (
                    index + 1,
                    instantiate(&directory, &invoice).await,
                    invoice.bindle.id,
                )
            }
        })
        .buffer_unordered(MAX_CONCURRENCY);

        // Note that these will print somewhat out-of-order with respect to `index` due to the `buffer_unordered`
        // call above.  If that becomes annoying, we can either stop using `buffer_unordered` or accumulate the
        // results and sort them before printing.
        let mut failed_ids = String::new();
        while let Some((index, result, id)) = stream.next().await {
            print!("({index:>5} of {total}) app {id} ({}): ", id.sha());
            if let Err(e) = result {
                failed_ids.push_str(&format!("{id}\n"));
                println!("failed: {e:?}");
            } else {
                println!("success!");
            }
        }
        if let Some(failures) = self.failures.as_ref() {
            if let Err(e) = std::fs::write(failures, failed_ids) {
                eprintln!(
                    "failed to write failed app ids to file at '{}': {e}",
                    failures.display()
                )
            }
        }

        Ok(())
    }
}

async fn read_id_list(path: &Option<PathBuf>) -> Result<Option<HashSet<String>>, Error> {
    let Some(path) = path else { return Ok(None) };
    Ok(Some(
        tokio::fs::read_to_string(path)
            .await
            .with_context(|| path.display().to_string())?
            .lines()
            .map(|s| s.to_owned())
            .collect::<HashSet<_>>(),
    ))
}

async fn instantiate(bindle_path: &Path, invoice: &Invoice) -> Result<()> {
    let working_dir = tempfile::tempdir().context("unable to create temporary directory")?;
    let working_dir = working_dir.path().canonicalize().with_context(|| {
        format!(
            "unable to canonicalize working directory path '{}'",
            working_dir.path().display()
        )
    })?;

    let manifest_path = bindle_path
        .join("parcels")
        .join(spin_loader::bindle::find_manifest(invoice)?)
        .join("parcel.dat");

    let raw = toml::from_str::<RawAppManifest>(str::from_utf8(
        &tokio::fs::read(&manifest_path)
            .await
            .with_context(|| format!("failed to read manifest at '{}'", manifest_path.display()))?,
    )?)?;

    let component_triggers = raw
        .components
        .iter()
        .map(|raw| (raw.id.clone(), raw.trigger.clone()))
        .collect();

    // Note that we ignore static assets, config variables, etc. since we're not running these components, just
    // instantiating them.
    let app = Application {
        info: ApplicationInformation {
            spin_version: SpinVersion::V1,
            name: invoice.bindle.id.name().to_string(),
            version: invoice.bindle.id.version_string(),
            description: invoice.bindle.description.clone(),
            authors: invoice.bindle.authors.clone().unwrap_or_default(),
            trigger: raw.trigger.clone(),
            origin: ApplicationOrigin::Bindle {
                id: invoice.bindle.id.to_string(),
                server: Url::from_file_path(bindle_path)
                    .map_err(|()| anyhow!("unable to convert path to URL"))?
                    .to_string(),
            },
        },
        variables: HashMap::new(),
        components: stream::iter(raw.components)
            .then(|raw| async move {
                let path = bindle_path
                    .join("parcels")
                    .join(&raw.source)
                    .join("parcel.dat");

                Ok::<_, Error>(CoreComponent {
                    source: ModuleSource::Buffer(
                        tokio::fs::read(&path).await.with_context(|| {
                            format!("failed to read parcel data at '{}'", path.display())
                        })?,
                        format!("parcel {}", raw.source),
                    ),
                    id: raw.id,
                    description: raw.description,
                    wasm: WasmConfig::default(),
                    config: raw.config.unwrap_or_default(),
                })
            })
            .try_collect()
            .await?,
        component_triggers,
    };

    let locked_app = spin_trigger::locked::build_locked_app(app, &working_dir)?;
    let locked_url = up::write_locked_app(&locked_app, &working_dir).await?;
    let loader = TriggerLoader::new(&working_dir, false);

    TriggerExecutorBuilder::<HttpTrigger>::new(loader)
        .build(
            locked_url,
            RuntimeConfig::new(None),
            HostComponentInitData::default(),
        )
        .await?
        .check()
        .await?;

    Ok(())
}
