use anyhow::{bail, Result};
use case_helper::Command;
use spin::http_types::{Method, Request, Response};
use std::{
    io::{self, Write},
    str,
};

#[macro_use]
mod wit {
    wit_bindgen::generate!({
        world: "reactor",
        path: "../wit-0.8",
        macro_call_prefix: "wit::",
    });
}
use wit::fermyon::spin::{self, postgres};
use wit::{exports::fermyon::spin as exports, fermyon::spin::mysql};

struct Spin;
export_reactor!(Spin);

impl exports::inbound_http::InboundHttp for Spin {
    fn handle_request(request: Request) -> Response {
        if request.method != Method::Post {
            Response {
                status: 405,
                headers: None,
                body: None,
            }
        } else if request.uri == "/" {
            dispatch(request.body)
        } else if request.uri != "/foo" {
            Response {
                status: 404,
                headers: None,
                body: None,
            }
        } else if request.headers != [("foo".into(), "bar".into())]
            || request.body.as_deref() != Some(b"Hello, SpinHttp!")
        {
            Response {
                status: 400,
                headers: None,
                body: None,
            }
        } else {
            Response {
                status: 200,
                headers: Some(vec![("lorem".into(), "ipsum".into())]),
                body: Some("dolor sit amet".as_bytes().to_owned()),
            }
        }
    }
}

impl exports::inbound_redis::InboundRedis for Spin {
    fn handle_message(_body: Vec<u8>) -> Result<(), spin::redis::Error> {
        Ok(())
    }
}

pub fn dispatch(body: Option<Vec<u8>>) -> Response {
    match execute(body) {
        Ok(()) => {
            _ = io::stdout().flush();
            _ = io::stderr().flush();

            Response {
                status: 200,
                headers: None,
                body: None,
            }
        }

        Err(e) => Response {
            status: 500,
            headers: None,
            body: Some(format!("{e:?}").into_bytes()),
        },
    }
}

fn execute(body: Option<Vec<u8>>) -> Result<()> {
    let command = Command::extract(body)?;
    match command {
        Command::Config { key } => {
            spin::config::get_config(&key)?;
        }

        Command::Http { url } => {
            spin::http::send_request(&Request {
                method: Method::Get,
                uri: url,
                headers: Vec::new(),
                params: Vec::new(),
                body: None,
            })?;
        }

        Command::RedisPublish {
            address,
            key,
            value,
        } => {
            spin::redis::publish(&address, &key, &value.into_bytes())?;
        }

        Command::RedisSet {
            address,
            key,
            value,
        } => {
            spin::redis::set(&address, &key, &value.into_bytes())?;
        }

        Command::RedisGet { address, key } => {
            spin::redis::get(&address, &key)?;
        }

        Command::RedisIncr { address, key } => {
            spin::redis::incr(&address, &key)?;
        }

        Command::RedisDel { address, keys } => {
            spin::redis::del(
                &address,
                &keys.iter().map(String::as_str).collect::<Vec<_>>(),
            )?;
        }

        Command::RedisSadd {
            address,
            key,
            params,
        } => {
            spin::redis::sadd(
                &address,
                &key,
                &params.iter().map(String::as_str).collect::<Vec<_>>(),
            )?;
        }

        Command::RedisSmembers { address, key } => {
            spin::redis::smembers(&address, &key)?;
        }

        Command::RedisSrem {
            address,
            key,
            params,
        } => {
            spin::redis::srem(
                &address,
                &key,
                &params.iter().map(String::as_str).collect::<Vec<_>>(),
            )?;
        }

        Command::RedisExecute {
            address,
            command,
            params,
        } => {
            let params: Vec<_> = params.into_iter().map(|s| s.into_bytes()).collect();
            spin::redis::execute(
                &address,
                &command,
                &params
                    .iter()
                    .map(|s| spin::redis_types::RedisParameter::Binary(s))
                    .collect::<Vec<_>>(),
            )?;
        }

        Command::PostgresExecute {
            address,
            statement,
            params,
        } => {
            postgres::execute(
                &address,
                &statement,
                &params
                    .iter()
                    .map(|param| parse_pg(param))
                    .collect::<Result<Vec<_>>>()?,
            )?;
        }

        Command::PostgresQuery {
            address,
            statement,
            params,
        } => {
            postgres::query(
                &address,
                &statement,
                &params
                    .iter()
                    .map(|param| parse_pg(param))
                    .collect::<Result<Vec<_>>>()?,
            )?;
        }

        Command::MysqlExecute {
            address,
            statement,
            params,
        } => {
            mysql::execute(
                &address,
                &statement,
                &params
                    .iter()
                    .map(|param| parse_mysql(param))
                    .collect::<Result<Vec<_>>>()?,
            )?;
        }

        Command::MysqlQuery {
            address,
            statement,
            params,
        } => {
            spin::mysql::query(
                &address,
                &statement,
                &params
                    .iter()
                    .map(|param| parse_mysql(param))
                    .collect::<Result<Vec<_>>>()?,
            )?;
        }

        Command::KeyValueOpen { name } => {
            spin::key_value::open(&name)?;
        }

        Command::KeyValueGet { store, key } => {
            spin::key_value::get(store, &key)?;
        }

        Command::KeyValueSet { store, key, value } => {
            spin::key_value::set(store, &key, value.as_bytes())?;
        }

        Command::KeyValueDelete { store, key } => {
            spin::key_value::delete(store, &key)?;
        }

        Command::KeyValueExists { store, key } => {
            spin::key_value::exists(store, &key)?;
        }

        Command::KeyValueGetKeys { store } => {
            spin::key_value::get_keys(store)?;
        }

        Command::KeyValueClose { store } => {
            spin::key_value::close(store);
        }
        Command::LlmInfer { model, prompt } => {
            let _ = spin::llm::infer(&model, &prompt, None);
        }

        Command::WasiEnv { key } => Command::env(key)?,
        Command::WasiEpoch => Command::epoch()?,
        Command::WasiRandom => Command::random()?,
        Command::WasiStdio => Command::stdio()?,
        Command::WasiRead { file_name } => Command::read(file_name)?,
        Command::WasiReaddir { dir_name } => Command::read_dir(dir_name)?,
        Command::WasiStat { file_name } => Command::stat(file_name)?,
    }

    Ok(())
}

fn parse_pg(param: &str) -> Result<spin::postgres::ParameterValue> {
    use spin::postgres::ParameterValue as PV;

    Ok(if param == "null" {
        PV::DbNull
    } else {
        let (type_, value) = case_helper::split_param(param)?;

        match type_ {
            "boolean" => PV::Boolean(value.parse()?),
            "int8" => PV::Int8(value.parse()?),
            "int16" => PV::Int16(value.parse()?),
            "int32" => PV::Int32(value.parse()?),
            "int64" => PV::Int64(value.parse()?),
            "uint8" => PV::Uint8(value.parse()?),
            "uint16" => PV::Uint16(value.parse()?),
            "uint32" => PV::Uint32(value.parse()?),
            "uint64" => PV::Uint64(value.parse()?),
            "floating32" => PV::Floating32(value.parse()?),
            "floating64" => PV::Floating64(value.parse()?),
            "str" => PV::Str(value),
            "binary" => PV::Binary(value.as_bytes()),
            _ => bail!("unknown parameter type: {type_}"),
        }
    })
}

fn parse_mysql(param: &str) -> Result<spin::mysql::ParameterValue> {
    use spin::mysql::ParameterValue as PV;

    Ok(if param == "null" {
        PV::DbNull
    } else {
        let (type_, value) = case_helper::split_param(param)?;

        match type_ {
            "boolean" => PV::Boolean(value.parse()?),
            "int8" => PV::Int8(value.parse()?),
            "int16" => PV::Int16(value.parse()?),
            "int32" => PV::Int32(value.parse()?),
            "int64" => PV::Int64(value.parse()?),
            "uint8" => PV::Uint8(value.parse()?),
            "uint16" => PV::Uint16(value.parse()?),
            "uint32" => PV::Uint32(value.parse()?),
            "uint64" => PV::Uint64(value.parse()?),
            "floating32" => PV::Floating32(value.parse()?),
            "floating64" => PV::Floating64(value.parse()?),
            "str" => PV::Str(value),
            "binary" => PV::Binary(value.as_bytes()),
            _ => bail!("unknown parameter type: {type_}"),
        }
    })
}
