#![allow(dead_code)]
use anyhow::{anyhow, Result};
use spin_sdk::{
    http::{Request, Response},
    http_component,
    pg,
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

impl TryFrom<&pg::Row> for Article {
    type Error = anyhow::Error;

    fn try_from(row: &pg::Row) -> Result<Self, Self::Error> {
        let id: i32 = (&row[0]).try_into()?;
        let title: String = (&row[1]).try_into()?;
        let content: String = (&row[2]).try_into()?;
        let authorname: String = (&row[3]).try_into()?;

        Ok(Self {
            id,
            title,
            content,
            authorname,
        })
    }
}

#[http_component]
fn process(req: Request) -> Result<Response> {
    match req.uri().path() {
        "/read" => read(req),
        "/write" => write(req),
        "/pg_backend_pid" => pg_backend_pid(req),
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
        let article = Article::try_from(&row)?;

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
    let count: i64 = (&row[0]).try_into()?;
    let response = format!("Count: {}\n", count);

    Ok(http::Response::builder()
        .status(200)
        .body(Some(response.into()))?)
}

fn pg_backend_pid(_req: Request) -> Result<Response> {
    let address = std::env::var(DB_URL_ENV)?;
    let sql = "SELECT pg_backend_pid()";

    let get_pid = || {
        let rowset = pg::query(&address, sql, &[])
            .map_err(|e| anyhow!("Error executing Postgres query: {:?}", e))?;

        let row = &rowset.rows[0];

        i32::try_from(&row[0])
    };

    assert_eq!(get_pid()?, get_pid()?);

    let response = format!("pg_backend_pid: {}\n", get_pid()?);

    Ok(http::Response::builder()
        .status(200)
        .body(Some(response.into()))?)
}

fn format_col(column: &pg::Column) -> String {
    format!("{}:{:?}", column.name, column.data_type)
}
