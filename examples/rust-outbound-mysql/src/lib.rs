use anyhow::{anyhow, Result};
use http::{HeaderValue, Method, Request, Response};
use spin_sdk::{
    http::Json,
    http_component,
    mysql::{self, ParameterValue},
};
use std::{collections::HashMap, str::FromStr};

use crate::model::as_pet;

mod model;

// The environment variable set in `spin.toml` that points to the
// address of the MySQL server that the component will write to
const DB_URL_ENV: &str = "DB_URL";

enum RequestAction {
    List,
    Get(i32),
    Create(String, Option<String>, bool),
    Error(u16),
}

#[http_component]
fn rust_outbound_mysql(
    req: Request<Json<HashMap<String, String>>>,
) -> Result<Response<Option<String>>> {
    match parse_request(req) {
        RequestAction::List => list(),
        RequestAction::Get(id) => get(id),
        RequestAction::Create(name, prey, is_finicky) => create(name, prey, is_finicky),
        RequestAction::Error(status) => error(status),
    }
}

fn parse_request(req: Request<Json<HashMap<String, String>>>) -> RequestAction {
    match *req.method() {
        Method::GET => match req.headers().get("spin-path-info") {
            None => RequestAction::Error(500),
            Some(header_val) => match header_val_to_int(header_val) {
                Ok(None) => RequestAction::List,
                Ok(Some(id)) => RequestAction::Get(id),
                Err(()) => RequestAction::Error(404),
            },
        },
        Method::POST => {
            let map = req.body();
            let name = match map.get("name") {
                Some(n) => n.to_owned(),
                None => return RequestAction::Error(400), // If this were a real app it would have error messages
            };
            let prey = map.get("prey").cloned();
            let is_finicky = map
                .get("is_finicky")
                .map(|s| s == "true")
                .unwrap_or_default();
            RequestAction::Create(name, prey, is_finicky)
        }
        _ => RequestAction::Error(405),
    }
}

fn header_val_to_int(header_val: &HeaderValue) -> Result<Option<i32>, ()> {
    match header_val.to_str() {
        Ok(path) => {
            let path_parts = &(path.split('/').skip(1).collect::<Vec<_>>()[..]);
            match *path_parts {
                [""] => Ok(None),
                [id_str] => match i32::from_str(id_str) {
                    Ok(id) => Ok(Some(id)),
                    Err(_) => Err(()),
                },
                _ => Err(()),
            }
        }
        Err(_) => Err(()),
    }
}

fn list() -> Result<Response<Option<String>>> {
    let address = std::env::var(DB_URL_ENV)?;
    let conn = mysql::Connection::open(&address)?;

    let sql = "SELECT id, name, prey, is_finicky FROM pets";
    let rowset = conn.query(sql, &[])?;

    let column_summary = rowset
        .columns
        .iter()
        .map(format_col)
        .collect::<Vec<_>>()
        .join(", ");

    let mut response_lines = vec![];

    for row in rowset.rows {
        let pet = as_pet(&row);
        println!("{:#?}", pet);
        response_lines.push(format!("{:#?}", pet));
    }

    let response = format!(
        "Found {} pet(s) as follows:\n{}\n\n(Column info: {})\n",
        response_lines.len(),
        response_lines.join("\n"),
        column_summary,
    );

    Ok(http::Response::builder().status(200).body(Some(response))?)
}

fn get(id: i32) -> Result<Response<Option<String>>> {
    let address = std::env::var(DB_URL_ENV)?;
    let conn = mysql::Connection::open(&address)?;

    let sql = "SELECT id, name, prey, is_finicky FROM pets WHERE id = ?";
    let params = vec![ParameterValue::Int32(id)];
    let rowset = conn.query(sql, &params)?;

    match rowset.rows.first() {
        None => Ok(http::Response::builder().status(404).body(None)?),
        Some(row) => {
            let pet = as_pet(row)?;
            let response = format!("{:?}", pet);
            Ok(http::Response::builder().status(200).body(Some(response))?)
        }
    }
}

fn create(
    name: String,
    prey: Option<String>,
    is_finicky: bool,
) -> Result<Response<Option<String>>> {
    let address = std::env::var(DB_URL_ENV)?;
    let conn = mysql::Connection::open(&address)?;

    let id = max_pet_id(&conn)? + 1;

    let prey_param = match prey {
        None => ParameterValue::DbNull,
        Some(str) => ParameterValue::Str(str),
    };

    let is_finicky_param = ParameterValue::Int8(i8::from(is_finicky));

    let sql = "INSERT INTO pets (id, name, prey, is_finicky) VALUES (?, ?, ?, ?)";
    let params = vec![
        ParameterValue::Int32(id),
        ParameterValue::Str(name),
        prey_param,
        is_finicky_param,
    ];
    conn.execute(sql, &params)?;

    let location_url = format!("/{}", id);

    Ok(http::Response::builder()
        .status(201)
        .header("Location", location_url)
        .body(None)?)
}

fn error(status: u16) -> Result<Response<Option<String>>> {
    Ok(http::Response::builder().status(status).body(None)?)
}

fn format_col(column: &mysql::Column) -> String {
    format!("{}: {:?}", column.name, column.data_type)
}

fn max_pet_id(conn: &mysql::Connection) -> Result<i32> {
    let sql = "SELECT MAX(id) FROM pets";
    let rowset = conn.query(sql, &[])?;

    match rowset.rows.first() {
        None => Ok(0),
        Some(row) => match row.first() {
            None => Ok(0),
            Some(mysql::DbValue::Int32(i)) => Ok(*i),
            Some(other) => Err(anyhow!(
                "Unexpected non-integer ID {:?}, can't insert",
                other
            )),
        },
    }
}
