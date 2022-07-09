use anyhow::{anyhow, Result};
use spin_sdk::{
    http::{internal_server_error, Request, Response},
    http_component, pg,
};

// The environment variable set in `spin.toml` that points to the
// address of the Pg server that the component will write to
const DB_URL_ENV: &str = "DB_URL";

#[http_component]
fn read(_req: Request) -> Result<Response> {
    let address = std::env::var(DB_URL_ENV)?;

    let sql = "select * from articletest";
    let rows = pg::query(address,sql, &vec![]).map_err(|_| anyhow!("Error execute pg command"))?;

    println!("rows: {:?}", rows);

    Ok(http::Response::builder().status(200).body(None)?)
}
/*
fn write(_req: Request) -> Result<Response> {
    let address = std::env::var(DB_URL_ENV)?;

    let sql = "insert into articletest values ('aaa', 'bbb', 'ccc')";
    let nrow_executed = pg::execute(address, sql, &vec![]).map_err(|_| anyhow!("Error execute pg command"))?;

    println!("nrow_executed: {}", nrow_executed);

    Ok(http::Response::builder().status(200).body(None)?)
}
*/
