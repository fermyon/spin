use anyhow::Context as _;
use spin_factor_sqlite::SqliteFactor;
use spin_factors::RuntimeFactors;
use spin_factors_executor::ExecutorHooks;

/// The default sqlite label
const DEFAULT_SQLITE_LABEL: &str = "default";

/// ExecutorHook for executing sqlite statements.
///
/// This executor assumes that the configured app has access to `SqliteFactor`.
/// It will silently ignore the hook if the app does not have access to `SqliteFactor`.
pub struct SqlStatementExecutorHook {
    sql_statements: Vec<String>,
}

impl SqlStatementExecutorHook {
    /// Creates a new SqlStatementExecutorHook
    ///
    /// The statements can be either a list of raw SQL statements or a list of `@{file:label}` statements.
    pub fn new(sql_statements: Vec<String>) -> Self {
        Self { sql_statements }
    }
}

impl<F: RuntimeFactors, U> ExecutorHooks<F, U> for SqlStatementExecutorHook {
    fn configure_app(
        &mut self,
        configured_app: &spin_factors::ConfiguredApp<F>,
    ) -> anyhow::Result<()> {
        if self.sql_statements.is_empty() {
            return Ok(());
        }
        let Some(sqlite) = configured_app.app_state::<SqliteFactor>().ok() else {
            return Ok(());
        };
        if let Ok(current) = tokio::runtime::Handle::try_current() {
            let _ = current.spawn(execute(sqlite.clone(), self.sql_statements.clone()));
        }
        Ok(())
    }
}

/// Executes the sql statements.
pub async fn execute(
    sqlite: spin_factor_sqlite::AppState,
    sql_statements: Vec<String>,
) -> anyhow::Result<()> {
    let get_database = |label| {
        let sqlite = &sqlite;
        async move {
            sqlite
                .get_connection(label)
                .await
                .transpose()
                .with_context(|| format!("failed connect to database with label '{label}'"))
        }
    };

    for statement in &sql_statements {
        if let Some(config) = statement.strip_prefix('@') {
            let (file, label) = parse_file_and_label(config)?;
            let database = get_database(label).await?.with_context(|| {
                    format!(
                        "based on the '@{config}' a registered database named '{label}' was expected but not found."
                    )
                })?;
            let sql = std::fs::read_to_string(file).with_context(|| {
                format!("could not read file '{file}' containing sql statements")
            })?;
            database.execute_batch(&sql).await.with_context(|| {
                format!("failed to execute sql against database '{label}' from file '{file}'")
            })?;
        } else {
            let Some(default) = get_database(DEFAULT_SQLITE_LABEL).await? else {
                debug_assert!(false, "the '{DEFAULT_SQLITE_LABEL}' sqlite database should always be available but for some reason was not");
                return Ok(());
            };
            default
                    .query(statement, Vec::new())
                    .await
                    .with_context(|| format!("failed to execute following sql statement against default database: '{statement}'"))?;
        }
    }
    Ok(())
}

/// Parses a @{file:label} sqlite statement
fn parse_file_and_label(config: &str) -> anyhow::Result<(&str, &str)> {
    let config = config.trim();
    let (file, label) = match config.split_once(':') {
        Some((_, label)) if label.trim().is_empty() => {
            anyhow::bail!("database label is empty in the '@{config}' sqlite statement")
        }
        Some((file, label)) => (file.trim(), label.trim()),
        None => (config, "default"),
    };
    Ok((file, label))
}
