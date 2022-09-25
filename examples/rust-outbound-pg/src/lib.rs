#![allow(dead_code)]
use anyhow::{anyhow, Result};
use spin_sdk::{
    http::{Request, Response},
    http_component, pg,
};

// The environment variable set in `spin.toml` that points to the
// address of the Pg server that the component will write to
const DB_URL_ENV: &str = "DB_URL";

#[derive(Debug, Clone)]
struct Article {
    id: i32,
    title: String,
    content: String,
    authorname: String,
}

#[http_component]
fn process(req: Request) -> Result<Response> {
    match req.uri().path() {
        "/read" => read(req),
        "/write" => write(req),
        _ => Ok(http::Response::builder()
            .status(404)
            .body(Some("Not found".into()))?),
    }
}

fn read(_req: Request) -> Result<Response> {
    let address = std::env::var(DB_URL_ENV)?;

    let sql = "SELECT id, title, content, authorname FROM articletest";
    let rowset = pg::query(&address, sql, &[])
        .map_err(|e| anyhow!("Error executing Postgres query: {:?}", e))?;

    let column_summary = rowset
        .columns
        .iter()
        .map(format_col)
        .collect::<Vec<_>>()
        .join(", ");

    let mut response_lines = vec![];

    for row in rowset.rows {
        let id = as_int(&row[0])?;
        let title = as_owned_string(&row[1])?;
        let content = as_owned_string(&row[2])?;
        let authorname = as_owned_string(&row[3])?;

        let article = Article {
            id,
            title,
            content,
            authorname,
        };

        println!("article: {:#?}", article);
        response_lines.push(format!("article: {:#?}", article));
    }

    // use it in business logic

    let response = format!(
        "Found {} article(s) as follows:\n{}\n\n(Column info: {})\n",
        response_lines.len(),
        response_lines.join("\n"),
        column_summary,
    );

    Ok(http::Response::builder()
        .status(200)
        .body(Some(response.into()))?)
}

fn write(_req: Request) -> Result<Response> {
    let address = std::env::var(DB_URL_ENV)?;

    let sql = "INSERT INTO articletest (title, content, authorname) VALUES ('aaa', 'bbb', 'ccc')";
    let nrow_executed =
        pg::execute(&address, sql, &[]).map_err(|_| anyhow!("Error execute pg command"))?;

    println!("nrow_executed: {}", nrow_executed);

    let sql = "SELECT COUNT(id) FROM articletest";
    let rowset = pg::query(&address, sql, &[])
        .map_err(|e| anyhow!("Error executing Postgres query: {:?}", e))?;
    let row = &rowset.rows[0];
    let count = as_bigint(&row[0])?;
    let response = format!("Count: {}\n", count);

    Ok(http::Response::builder()
        .status(200)
        .body(Some(response.into()))?)
}

fn as_owned_string(value: &pg::DbValue) -> anyhow::Result<String> {
    match value {
        pg::DbValue::Str(s) => Ok(s.to_owned()),
        _ => Err(anyhow!("Expected string from database but got {:?}", value)),
    }
}

fn as_int(value: &pg::DbValue) -> anyhow::Result<i32> {
    match value {
        pg::DbValue::Int32(n) => Ok(*n),
        _ => Err(anyhow!(
            "Expected integer from database but got {:?}",
            value
        )),
    }
}

fn as_bigint(value: &pg::DbValue) -> anyhow::Result<i64> {
    match value {
        pg::DbValue::Int64(n) => Ok(*n),
        _ => Err(anyhow!(
            "Expected integer from database but got {:?}",
            value
        )),
    }
}

fn format_col(column: &pg::Column) -> String {
    format!("{}:{:?}", column.name, column.data_type)
}
