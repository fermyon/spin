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
    title: String, 
    content: String,
    authorname: String,
}

#[http_component]
fn read(_req: Request) -> Result<Response> {
    let address = std::env::var(DB_URL_ENV)?;

    let sql = "select title, content, authorname from articletest";
    let rows = pg::query(&address,sql, &[]).map_err(|_| anyhow!("Error execute pg command"))?;

    println!("rows: {:?}", rows);
    for row in rows {
        let title = String::from_utf8(row[0].clone())?;
        let content = String::from_utf8(row[1].clone())?;
        let authorname = String::from_utf8(row[2].clone())?;

        let article = Article {title, content, authorname};

        println!("article: {:#?}", article);
    }
    
    // use it in business logic

    Ok(http::Response::builder().status(200).body(None)?)
}
/*
fn write(_req: Request) -> Result<Response> {
    let address = std::env::var(DB_URL_ENV)?;

    let sql = "insert into articletest values ('aaa', 'bbb', 'ccc')";
    let nrow_executed = pg::execute(&address, sql, &vec![]).map_err(|_| anyhow!("Error execute pg command"))?;

    println!("nrow_executed: {}", nrow_executed);

    Ok(http::Response::builder().status(200).body(None)?)
}
*/
