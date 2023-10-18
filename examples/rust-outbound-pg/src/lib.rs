#![allow(dead_code)]
use anyhow::Result;
use http::{Request, Response};
use spin_sdk::{
    http_component,
    pg::{self, Decode},
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
    coauthor: Option<String>,
}

impl TryFrom<&pg::Row> for Article {
    type Error = anyhow::Error;

    fn try_from(row: &pg::Row) -> Result<Self, Self::Error> {
        let id = i32::decode(&row[0])?;
        let title = String::decode(&row[1])?;
        let content = String::decode(&row[2])?;
        let authorname = String::decode(&row[3])?;
        let coauthor = Option::<String>::decode(&row[4])?;

        Ok(Self {
            id,
            title,
            content,
            authorname,
            coauthor,
        })
    }
}

#[http_component]
fn process(req: Request<()>) -> Result<Response<String>> {
    match req.uri().path() {
        "/read" => read(req),
        "/write" => write(req),
        "/pg_backend_pid" => pg_backend_pid(req),
        _ => Ok(http::Response::builder()
            .status(404)
            .body("Not found".into())?),
    }
}

fn read(_req: Request<()>) -> Result<Response<String>> {
    let address = std::env::var(DB_URL_ENV)?;
    let conn = pg::Connection::open(&address)?;

    let sql = "SELECT id, title, content, authorname, coauthor FROM articletest";
    let rowset = conn.query(sql, &[])?;

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

    Ok(http::Response::builder().status(200).body(response)?)
}

fn write(_req: Request<()>) -> Result<Response<String>> {
    let address = std::env::var(DB_URL_ENV)?;
    let conn = pg::Connection::open(&address)?;

    let sql = "INSERT INTO articletest (title, content, authorname) VALUES ('aaa', 'bbb', 'ccc')";
    let nrow_executed = conn.execute(sql, &[])?;

    println!("nrow_executed: {}", nrow_executed);

    let sql = "SELECT COUNT(id) FROM articletest";
    let rowset = conn.query(sql, &[])?;
    let row = &rowset.rows[0];
    let count = i64::decode(&row[0])?;
    let response = format!("Count: {}\n", count);

    Ok(http::Response::builder().status(200).body(response)?)
}

fn pg_backend_pid(_req: Request<()>) -> Result<Response<String>> {
    let address = std::env::var(DB_URL_ENV)?;
    let conn = pg::Connection::open(&address)?;
    let sql = "SELECT pg_backend_pid()";

    let get_pid = || {
        let rowset = conn.query(sql, &[])?;
        let row = &rowset.rows[0];

        i32::decode(&row[0])
    };

    assert_eq!(get_pid()?, get_pid()?);

    let response = format!("pg_backend_pid: {}\n", get_pid()?);

    Ok(http::Response::builder().status(200).body(response)?)
}

fn format_col(column: &pg::Column) -> String {
    format!("{}:{:?}", column.name, column.data_type)
}
