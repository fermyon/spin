use super::*;

mod rdbms_types {
    use super::*;

    impl From<v2::rdbms_types::Column> for v1::rdbms_types::Column {
        fn from(value: v2::rdbms_types::Column) -> Self {
            v1::rdbms_types::Column {
                name: value.name,
                data_type: value.data_type.into(),
            }
        }
    }

    impl From<spin::postgres::postgres::Column> for v1::rdbms_types::Column {
        fn from(value: spin::postgres::postgres::Column) -> Self {
            v1::rdbms_types::Column {
                name: value.name,
                data_type: value.data_type.into(),
            }
        }
    }

    impl From<spin::postgres::postgres::Column> for v2::rdbms_types::Column {
        fn from(value: spin::postgres::postgres::Column) -> Self {
            v2::rdbms_types::Column {
                name: value.name,
                data_type: value.data_type.into(),
            }
        }
    }

    impl From<v2::rdbms_types::DbValue> for v1::rdbms_types::DbValue {
        fn from(value: v2::rdbms_types::DbValue) -> v1::rdbms_types::DbValue {
            match value {
                v2::rdbms_types::DbValue::Boolean(b) => v1::rdbms_types::DbValue::Boolean(b),
                v2::rdbms_types::DbValue::Int8(i) => v1::rdbms_types::DbValue::Int8(i),
                v2::rdbms_types::DbValue::Int16(i) => v1::rdbms_types::DbValue::Int16(i),
                v2::rdbms_types::DbValue::Int32(i) => v1::rdbms_types::DbValue::Int32(i),
                v2::rdbms_types::DbValue::Int64(i) => v1::rdbms_types::DbValue::Int64(i),
                v2::rdbms_types::DbValue::Uint8(j) => v1::rdbms_types::DbValue::Uint8(j),
                v2::rdbms_types::DbValue::Uint16(u) => v1::rdbms_types::DbValue::Uint16(u),
                v2::rdbms_types::DbValue::Uint32(u) => v1::rdbms_types::DbValue::Uint32(u),
                v2::rdbms_types::DbValue::Uint64(u) => v1::rdbms_types::DbValue::Uint64(u),
                v2::rdbms_types::DbValue::Floating32(r) => v1::rdbms_types::DbValue::Floating32(r),
                v2::rdbms_types::DbValue::Floating64(r) => v1::rdbms_types::DbValue::Floating64(r),
                v2::rdbms_types::DbValue::Str(s) => v1::rdbms_types::DbValue::Str(s),
                v2::rdbms_types::DbValue::Binary(b) => v1::rdbms_types::DbValue::Binary(b),
                v2::rdbms_types::DbValue::DbNull => v1::rdbms_types::DbValue::DbNull,
                v2::rdbms_types::DbValue::Unsupported => v1::rdbms_types::DbValue::Unsupported,
            }
        }
    }

    impl From<spin::postgres::postgres::DbValue> for v1::rdbms_types::DbValue {
        fn from(value: spin::postgres::postgres::DbValue) -> v1::rdbms_types::DbValue {
            match value {
                spin::postgres::postgres::DbValue::Boolean(b) => {
                    v1::rdbms_types::DbValue::Boolean(b)
                }
                spin::postgres::postgres::DbValue::Int8(i) => v1::rdbms_types::DbValue::Int8(i),
                spin::postgres::postgres::DbValue::Int16(i) => v1::rdbms_types::DbValue::Int16(i),
                spin::postgres::postgres::DbValue::Int32(i) => v1::rdbms_types::DbValue::Int32(i),
                spin::postgres::postgres::DbValue::Int64(i) => v1::rdbms_types::DbValue::Int64(i),
                spin::postgres::postgres::DbValue::Floating32(r) => {
                    v1::rdbms_types::DbValue::Floating32(r)
                }
                spin::postgres::postgres::DbValue::Floating64(r) => {
                    v1::rdbms_types::DbValue::Floating64(r)
                }
                spin::postgres::postgres::DbValue::Str(s) => v1::rdbms_types::DbValue::Str(s),
                spin::postgres::postgres::DbValue::Binary(b) => v1::rdbms_types::DbValue::Binary(b),
                spin::postgres::postgres::DbValue::DbNull => v1::rdbms_types::DbValue::DbNull,
                spin::postgres::postgres::DbValue::Unsupported => {
                    v1::rdbms_types::DbValue::Unsupported
                }
                _ => v1::rdbms_types::DbValue::Unsupported,
            }
        }
    }

    impl From<spin::postgres::postgres::DbValue> for v2::rdbms_types::DbValue {
        fn from(value: spin::postgres::postgres::DbValue) -> v2::rdbms_types::DbValue {
            match value {
                spin::postgres::postgres::DbValue::Boolean(b) => {
                    v2::rdbms_types::DbValue::Boolean(b)
                }
                spin::postgres::postgres::DbValue::Int8(i) => v2::rdbms_types::DbValue::Int8(i),
                spin::postgres::postgres::DbValue::Int16(i) => v2::rdbms_types::DbValue::Int16(i),
                spin::postgres::postgres::DbValue::Int32(i) => v2::rdbms_types::DbValue::Int32(i),
                spin::postgres::postgres::DbValue::Int64(i) => v2::rdbms_types::DbValue::Int64(i),
                spin::postgres::postgres::DbValue::Floating32(r) => {
                    v2::rdbms_types::DbValue::Floating32(r)
                }
                spin::postgres::postgres::DbValue::Floating64(r) => {
                    v2::rdbms_types::DbValue::Floating64(r)
                }
                spin::postgres::postgres::DbValue::Str(s) => v2::rdbms_types::DbValue::Str(s),
                spin::postgres::postgres::DbValue::Binary(b) => v2::rdbms_types::DbValue::Binary(b),
                spin::postgres::postgres::DbValue::DbNull => v2::rdbms_types::DbValue::DbNull,
                spin::postgres::postgres::DbValue::Unsupported => {
                    v2::rdbms_types::DbValue::Unsupported
                }
                _ => v2::rdbms_types::DbValue::Unsupported,
            }
        }
    }

    impl From<spin::postgres::postgres::DbDataType> for v1::rdbms_types::DbDataType {
        fn from(value: spin::postgres::postgres::DbDataType) -> v1::rdbms_types::DbDataType {
            match value {
                spin::postgres::postgres::DbDataType::Boolean => {
                    v1::rdbms_types::DbDataType::Boolean
                }
                spin::postgres::postgres::DbDataType::Int8 => v1::rdbms_types::DbDataType::Int8,
                spin::postgres::postgres::DbDataType::Int16 => v1::rdbms_types::DbDataType::Int16,
                spin::postgres::postgres::DbDataType::Int32 => v1::rdbms_types::DbDataType::Int32,
                spin::postgres::postgres::DbDataType::Int64 => v1::rdbms_types::DbDataType::Int64,
                spin::postgres::postgres::DbDataType::Floating32 => {
                    v1::rdbms_types::DbDataType::Floating32
                }
                spin::postgres::postgres::DbDataType::Floating64 => {
                    v1::rdbms_types::DbDataType::Floating64
                }
                spin::postgres::postgres::DbDataType::Str => v1::rdbms_types::DbDataType::Str,
                spin::postgres::postgres::DbDataType::Binary => v1::rdbms_types::DbDataType::Binary,
                spin::postgres::postgres::DbDataType::Other => v1::rdbms_types::DbDataType::Other,
                _ => v1::rdbms_types::DbDataType::Other,
            }
        }
    }

    impl From<spin::postgres::postgres::DbDataType> for v2::rdbms_types::DbDataType {
        fn from(value: spin::postgres::postgres::DbDataType) -> v2::rdbms_types::DbDataType {
            match value {
                spin::postgres::postgres::DbDataType::Boolean => {
                    v2::rdbms_types::DbDataType::Boolean
                }
                spin::postgres::postgres::DbDataType::Int8 => v2::rdbms_types::DbDataType::Int8,
                spin::postgres::postgres::DbDataType::Int16 => v2::rdbms_types::DbDataType::Int16,
                spin::postgres::postgres::DbDataType::Int32 => v2::rdbms_types::DbDataType::Int32,
                spin::postgres::postgres::DbDataType::Int64 => v2::rdbms_types::DbDataType::Int64,
                spin::postgres::postgres::DbDataType::Floating32 => {
                    v2::rdbms_types::DbDataType::Floating32
                }
                spin::postgres::postgres::DbDataType::Floating64 => {
                    v2::rdbms_types::DbDataType::Floating64
                }
                spin::postgres::postgres::DbDataType::Str => v2::rdbms_types::DbDataType::Str,
                spin::postgres::postgres::DbDataType::Binary => v2::rdbms_types::DbDataType::Binary,
                spin::postgres::postgres::DbDataType::Other => v2::rdbms_types::DbDataType::Other,
                _ => v2::rdbms_types::DbDataType::Other,
            }
        }
    }

    impl From<v2::rdbms_types::DbDataType> for v1::rdbms_types::DbDataType {
        fn from(value: v2::rdbms_types::DbDataType) -> v1::rdbms_types::DbDataType {
            match value {
                v2::rdbms_types::DbDataType::Boolean => v1::rdbms_types::DbDataType::Boolean,
                v2::rdbms_types::DbDataType::Int8 => v1::rdbms_types::DbDataType::Int8,
                v2::rdbms_types::DbDataType::Int16 => v1::rdbms_types::DbDataType::Int16,
                v2::rdbms_types::DbDataType::Int32 => v1::rdbms_types::DbDataType::Int32,
                v2::rdbms_types::DbDataType::Int64 => v1::rdbms_types::DbDataType::Int64,
                v2::rdbms_types::DbDataType::Uint8 => v1::rdbms_types::DbDataType::Uint8,
                v2::rdbms_types::DbDataType::Uint16 => v1::rdbms_types::DbDataType::Uint16,
                v2::rdbms_types::DbDataType::Uint32 => v1::rdbms_types::DbDataType::Uint32,
                v2::rdbms_types::DbDataType::Uint64 => v1::rdbms_types::DbDataType::Uint64,
                v2::rdbms_types::DbDataType::Floating32 => v1::rdbms_types::DbDataType::Floating32,
                v2::rdbms_types::DbDataType::Floating64 => v1::rdbms_types::DbDataType::Floating64,
                v2::rdbms_types::DbDataType::Str => v1::rdbms_types::DbDataType::Str,
                v2::rdbms_types::DbDataType::Binary => v1::rdbms_types::DbDataType::Binary,
                v2::rdbms_types::DbDataType::Other => v1::rdbms_types::DbDataType::Other,
            }
        }
    }

    impl From<v1::rdbms_types::ParameterValue> for v2::rdbms_types::ParameterValue {
        fn from(value: v1::rdbms_types::ParameterValue) -> v2::rdbms_types::ParameterValue {
            match value {
                v1::rdbms_types::ParameterValue::Boolean(b) => {
                    v2::rdbms_types::ParameterValue::Boolean(b)
                }
                v1::rdbms_types::ParameterValue::Int8(i) => {
                    v2::rdbms_types::ParameterValue::Int8(i)
                }
                v1::rdbms_types::ParameterValue::Int16(i) => {
                    v2::rdbms_types::ParameterValue::Int16(i)
                }
                v1::rdbms_types::ParameterValue::Int32(i) => {
                    v2::rdbms_types::ParameterValue::Int32(i)
                }
                v1::rdbms_types::ParameterValue::Int64(i) => {
                    v2::rdbms_types::ParameterValue::Int64(i)
                }
                v1::rdbms_types::ParameterValue::Uint8(u) => {
                    v2::rdbms_types::ParameterValue::Uint8(u)
                }
                v1::rdbms_types::ParameterValue::Uint16(u) => {
                    v2::rdbms_types::ParameterValue::Uint16(u)
                }
                v1::rdbms_types::ParameterValue::Uint32(u) => {
                    v2::rdbms_types::ParameterValue::Uint32(u)
                }
                v1::rdbms_types::ParameterValue::Uint64(u) => {
                    v2::rdbms_types::ParameterValue::Uint64(u)
                }
                v1::rdbms_types::ParameterValue::Floating32(r) => {
                    v2::rdbms_types::ParameterValue::Floating32(r)
                }
                v1::rdbms_types::ParameterValue::Floating64(r) => {
                    v2::rdbms_types::ParameterValue::Floating64(r)
                }
                v1::rdbms_types::ParameterValue::Str(s) => v2::rdbms_types::ParameterValue::Str(s),
                v1::rdbms_types::ParameterValue::Binary(b) => {
                    v2::rdbms_types::ParameterValue::Binary(b)
                }
                v1::rdbms_types::ParameterValue::DbNull => v2::rdbms_types::ParameterValue::DbNull,
            }
        }
    }

    impl TryFrom<v1::rdbms_types::ParameterValue> for spin::postgres::postgres::ParameterValue {
        type Error = v1::postgres::PgError;

        fn try_from(
            value: v1::rdbms_types::ParameterValue,
        ) -> Result<spin::postgres::postgres::ParameterValue, Self::Error> {
            let converted = match value {
                v1::rdbms_types::ParameterValue::Boolean(b) => {
                    spin::postgres::postgres::ParameterValue::Boolean(b)
                }
                v1::rdbms_types::ParameterValue::Int8(i) => {
                    spin::postgres::postgres::ParameterValue::Int8(i)
                }
                v1::rdbms_types::ParameterValue::Int16(i) => {
                    spin::postgres::postgres::ParameterValue::Int16(i)
                }
                v1::rdbms_types::ParameterValue::Int32(i) => {
                    spin::postgres::postgres::ParameterValue::Int32(i)
                }
                v1::rdbms_types::ParameterValue::Int64(i) => {
                    spin::postgres::postgres::ParameterValue::Int64(i)
                }
                v1::rdbms_types::ParameterValue::Uint8(_)
                | v1::rdbms_types::ParameterValue::Uint16(_)
                | v1::rdbms_types::ParameterValue::Uint32(_)
                | v1::rdbms_types::ParameterValue::Uint64(_) => {
                    return Err(v1::postgres::PgError::ValueConversionFailed(
                        "Postgres does not support unsigned integers".to_owned(),
                    ));
                }
                v1::rdbms_types::ParameterValue::Floating32(r) => {
                    spin::postgres::postgres::ParameterValue::Floating32(r)
                }
                v1::rdbms_types::ParameterValue::Floating64(r) => {
                    spin::postgres::postgres::ParameterValue::Floating64(r)
                }
                v1::rdbms_types::ParameterValue::Str(s) => {
                    spin::postgres::postgres::ParameterValue::Str(s)
                }
                v1::rdbms_types::ParameterValue::Binary(b) => {
                    spin::postgres::postgres::ParameterValue::Binary(b)
                }
                v1::rdbms_types::ParameterValue::DbNull => {
                    spin::postgres::postgres::ParameterValue::DbNull
                }
            };
            Ok(converted)
        }
    }

    impl TryFrom<v2::rdbms_types::ParameterValue> for spin::postgres::postgres::ParameterValue {
        type Error = v2::rdbms_types::Error;

        fn try_from(
            value: v2::rdbms_types::ParameterValue,
        ) -> Result<spin::postgres::postgres::ParameterValue, Self::Error> {
            let converted = match value {
                v2::rdbms_types::ParameterValue::Boolean(b) => {
                    spin::postgres::postgres::ParameterValue::Boolean(b)
                }
                v2::rdbms_types::ParameterValue::Int8(i) => {
                    spin::postgres::postgres::ParameterValue::Int8(i)
                }
                v2::rdbms_types::ParameterValue::Int16(i) => {
                    spin::postgres::postgres::ParameterValue::Int16(i)
                }
                v2::rdbms_types::ParameterValue::Int32(i) => {
                    spin::postgres::postgres::ParameterValue::Int32(i)
                }
                v2::rdbms_types::ParameterValue::Int64(i) => {
                    spin::postgres::postgres::ParameterValue::Int64(i)
                }
                v2::rdbms_types::ParameterValue::Uint8(_)
                | v2::rdbms_types::ParameterValue::Uint16(_)
                | v2::rdbms_types::ParameterValue::Uint32(_)
                | v2::rdbms_types::ParameterValue::Uint64(_) => {
                    return Err(v2::rdbms_types::Error::ValueConversionFailed(
                        "Postgres does not support unsigned integers".to_owned(),
                    ));
                }
                v2::rdbms_types::ParameterValue::Floating32(r) => {
                    spin::postgres::postgres::ParameterValue::Floating32(r)
                }
                v2::rdbms_types::ParameterValue::Floating64(r) => {
                    spin::postgres::postgres::ParameterValue::Floating64(r)
                }
                v2::rdbms_types::ParameterValue::Str(s) => {
                    spin::postgres::postgres::ParameterValue::Str(s)
                }
                v2::rdbms_types::ParameterValue::Binary(b) => {
                    spin::postgres::postgres::ParameterValue::Binary(b)
                }
                v2::rdbms_types::ParameterValue::DbNull => {
                    spin::postgres::postgres::ParameterValue::DbNull
                }
            };
            Ok(converted)
        }
    }

    impl From<v2::rdbms_types::Error> for v1::mysql::MysqlError {
        fn from(error: v2::rdbms_types::Error) -> v1::mysql::MysqlError {
            match error {
                v2::mysql::Error::ConnectionFailed(e) => v1::mysql::MysqlError::ConnectionFailed(e),
                v2::mysql::Error::BadParameter(e) => v1::mysql::MysqlError::BadParameter(e),
                v2::mysql::Error::QueryFailed(e) => v1::mysql::MysqlError::QueryFailed(e),
                v2::mysql::Error::ValueConversionFailed(e) => {
                    v1::mysql::MysqlError::ValueConversionFailed(e)
                }
                v2::mysql::Error::Other(e) => v1::mysql::MysqlError::OtherError(e),
            }
        }
    }

    impl From<spin::postgres::postgres::Error> for v1::postgres::PgError {
        fn from(error: spin::postgres::postgres::Error) -> v1::postgres::PgError {
            match error {
                spin::postgres::postgres::Error::ConnectionFailed(e) => {
                    v1::postgres::PgError::ConnectionFailed(e)
                }
                spin::postgres::postgres::Error::BadParameter(e) => {
                    v1::postgres::PgError::BadParameter(e)
                }
                spin::postgres::postgres::Error::QueryFailed(e) => {
                    v1::postgres::PgError::QueryFailed(e)
                }
                spin::postgres::postgres::Error::ValueConversionFailed(e) => {
                    v1::postgres::PgError::ValueConversionFailed(e)
                }
                spin::postgres::postgres::Error::Other(e) => v1::postgres::PgError::OtherError(e),
            }
        }
    }

    impl From<spin::postgres::postgres::Error> for v2::rdbms_types::Error {
        fn from(error: spin::postgres::postgres::Error) -> v2::rdbms_types::Error {
            match error {
                spin::postgres::postgres::Error::ConnectionFailed(e) => {
                    v2::rdbms_types::Error::ConnectionFailed(e)
                }
                spin::postgres::postgres::Error::BadParameter(e) => {
                    v2::rdbms_types::Error::BadParameter(e)
                }
                spin::postgres::postgres::Error::QueryFailed(e) => {
                    v2::rdbms_types::Error::QueryFailed(e)
                }
                spin::postgres::postgres::Error::ValueConversionFailed(e) => {
                    v2::rdbms_types::Error::ValueConversionFailed(e)
                }
                spin::postgres::postgres::Error::Other(e) => v2::rdbms_types::Error::Other(e),
            }
        }
    }
}

mod postgres {
    use super::*;

    impl From<spin::postgres::postgres::RowSet> for v1::postgres::RowSet {
        fn from(value: spin::postgres::postgres::RowSet) -> v1::postgres::RowSet {
            v1::mysql::RowSet {
                columns: value.columns.into_iter().map(Into::into).collect(),
                rows: value
                    .rows
                    .into_iter()
                    .map(|r| r.into_iter().map(Into::into).collect())
                    .collect(),
            }
        }
    }

    impl From<spin::postgres::postgres::RowSet> for v2::rdbms_types::RowSet {
        fn from(value: spin::postgres::postgres::RowSet) -> v2::rdbms_types::RowSet {
            v2::rdbms_types::RowSet {
                columns: value.columns.into_iter().map(Into::into).collect(),
                rows: value
                    .rows
                    .into_iter()
                    .map(|r| r.into_iter().map(Into::into).collect())
                    .collect(),
            }
        }
    }
}

mod mysql {
    use super::*;
    impl From<v2::mysql::RowSet> for v1::mysql::RowSet {
        fn from(value: v2::mysql::RowSet) -> v1::mysql::RowSet {
            v1::mysql::RowSet {
                columns: value.columns.into_iter().map(Into::into).collect(),
                rows: value
                    .rows
                    .into_iter()
                    .map(|r| r.into_iter().map(Into::into).collect())
                    .collect(),
            }
        }
    }
}

mod redis {
    use super::*;

    impl From<v1::redis::RedisParameter> for v2::redis::RedisParameter {
        fn from(value: v1::redis::RedisParameter) -> Self {
            match value {
                v1::redis::RedisParameter::Int64(i) => v2::redis::RedisParameter::Int64(i),
                v1::redis::RedisParameter::Binary(b) => v2::redis::RedisParameter::Binary(b),
            }
        }
    }

    impl From<v2::redis::RedisResult> for v1::redis::RedisResult {
        fn from(value: v2::redis::RedisResult) -> Self {
            match value {
                v2::redis::RedisResult::Nil => v1::redis::RedisResult::Nil,
                v2::redis::RedisResult::Status(s) => v1::redis::RedisResult::Status(s),
                v2::redis::RedisResult::Int64(i) => v1::redis::RedisResult::Int64(i),
                v2::redis::RedisResult::Binary(b) => v1::redis::RedisResult::Binary(b),
            }
        }
    }
}

mod llm {
    use super::*;

    impl From<v1::llm::InferencingParams> for v2::llm::InferencingParams {
        fn from(value: v1::llm::InferencingParams) -> Self {
            Self {
                max_tokens: value.max_tokens,
                repeat_penalty: value.repeat_penalty,
                repeat_penalty_last_n_token_count: value.repeat_penalty_last_n_token_count,
                temperature: value.temperature,
                top_k: value.top_k,
                top_p: value.top_p,
            }
        }
    }

    impl From<v2::llm::InferencingResult> for v1::llm::InferencingResult {
        fn from(value: v2::llm::InferencingResult) -> Self {
            Self {
                text: value.text,
                usage: v1::llm::InferencingUsage {
                    prompt_token_count: value.usage.prompt_token_count,
                    generated_token_count: value.usage.generated_token_count,
                },
            }
        }
    }

    impl From<v2::llm::EmbeddingsResult> for v1::llm::EmbeddingsResult {
        fn from(value: v2::llm::EmbeddingsResult) -> Self {
            Self {
                embeddings: value.embeddings,
                usage: v1::llm::EmbeddingsUsage {
                    prompt_token_count: value.usage.prompt_token_count,
                },
            }
        }
    }

    impl From<v2::llm::Error> for v1::llm::Error {
        fn from(value: v2::llm::Error) -> Self {
            match value {
                v2::llm::Error::ModelNotSupported => Self::ModelNotSupported,
                v2::llm::Error::RuntimeError(s) => Self::RuntimeError(s),
                v2::llm::Error::InvalidInput(s) => Self::InvalidInput(s),
            }
        }
    }
}

mod observe {
    use super::*;
    use opentelemetry::StringValue;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use wasi::observe::tracer;

    impl From<tracer::Value> for opentelemetry::Value {
        fn from(value: tracer::Value) -> Self {
            match value {
                tracer::Value::String(v) => v.into(),
                tracer::Value::Bool(v) => v.into(),
                tracer::Value::Float64(v) => v.into(),
                tracer::Value::S64(v) => v.into(),
                tracer::Value::StringArray(v) => opentelemetry::Value::Array(
                    v.into_iter()
                        .map(StringValue::from)
                        .collect::<Vec<_>>()
                        .into(),
                ),
                tracer::Value::BoolArray(v) => opentelemetry::Value::Array(v.into()),
                tracer::Value::Float64Array(v) => opentelemetry::Value::Array(v.into()),
                tracer::Value::S64Array(v) => opentelemetry::Value::Array(v.into()),
            }
        }
    }

    impl From<tracer::KeyValue> for opentelemetry::KeyValue {
        fn from(kv: tracer::KeyValue) -> Self {
            opentelemetry::KeyValue::new(kv.key, kv.value)
        }
    }

    impl From<tracer::TraceFlags> for opentelemetry::trace::TraceFlags {
        fn from(flags: tracer::TraceFlags) -> Self {
            Self::new(flags.as_array()[0] as u8)
        }
    }

    impl From<opentelemetry::trace::TraceFlags> for tracer::TraceFlags {
        fn from(flags: opentelemetry::trace::TraceFlags) -> Self {
            if flags.is_sampled() {
                tracer::TraceFlags::SAMPLED
            } else {
                tracer::TraceFlags::empty()
            }
        }
    }

    impl From<tracer::SpanContext> for opentelemetry::trace::SpanContext {
        fn from(sc: tracer::SpanContext) -> Self {
            // TODO(Reviewer): Should this be try_from instead an propagate this error out of the WIT?
            let trace_id = opentelemetry::trace::TraceId::from_hex(&sc.trace_id)
                .unwrap_or(opentelemetry::trace::TraceId::INVALID);
            let span_id = opentelemetry::trace::SpanId::from_hex(&sc.span_id)
                .unwrap_or(opentelemetry::trace::SpanId::INVALID);
            let trace_state = opentelemetry::trace::TraceState::from_key_value(sc.trace_state)
                .unwrap_or_else(|_| opentelemetry::trace::TraceState::default());
            Self::new(
                trace_id,
                span_id,
                sc.trace_flags.into(),
                sc.is_remote,
                trace_state,
            )
        }
    }

    impl From<opentelemetry::trace::SpanContext> for tracer::SpanContext {
        fn from(sc: opentelemetry::trace::SpanContext) -> Self {
            Self {
                trace_id: format!("{:x}", sc.trace_id()),
                span_id: format!("{:x}", sc.span_id()),
                trace_flags: sc.trace_flags().into(),
                is_remote: sc.is_remote(),
                trace_state: sc
                    .trace_state()
                    .header()
                    .split(',')
                    // TODO(Reviewer): Should this be try_from instead an propagate this error out of the WIT?
                    .filter_map(|s| {
                        if let Some((key, value)) = s.split_once('=') {
                            Some((key.to_string(), value.to_string()))
                        } else {
                            None
                        }
                    })
                    .collect(),
            }
        }
    }

    impl From<tracer::Status> for opentelemetry::trace::Status {
        fn from(status: tracer::Status) -> Self {
            match status {
                tracer::Status::Unset => Self::Unset,
                tracer::Status::Ok => Self::Ok,
                tracer::Status::Error(s) => Self::Error {
                    description: s.into(),
                },
            }
        }
    }

    impl From<tracer::SpanKind> for opentelemetry::trace::SpanKind {
        fn from(kind: tracer::SpanKind) -> Self {
            match kind {
                tracer::SpanKind::Client => opentelemetry::trace::SpanKind::Client,
                tracer::SpanKind::Server => opentelemetry::trace::SpanKind::Server,
                tracer::SpanKind::Producer => opentelemetry::trace::SpanKind::Producer,
                tracer::SpanKind::Consumer => opentelemetry::trace::SpanKind::Consumer,
                tracer::SpanKind::Internal => opentelemetry::trace::SpanKind::Internal,
            }
        }
    }

    impl From<tracer::Link> for opentelemetry::trace::Link {
        fn from(link: tracer::Link) -> Self {
            Self::new(
                link.span_context.into(),
                link.attributes.into_iter().map(Into::into).collect(),
                0,
            )
        }
    }

    impl From<tracer::Datetime> for SystemTime {
        fn from(timestamp: tracer::Datetime) -> Self {
            UNIX_EPOCH
                + Duration::from_secs(timestamp.seconds)
                + Duration::from_nanos(timestamp.nanoseconds as u64)
        }
    }

    #[allow(clippy::derivable_impls)]
    impl Default for tracer::StartOptions {
        fn default() -> Self {
            Self {
                new_root: false,
                span_kind: None,
                attributes: None,
                links: None,
                timestamp: None,
            }
        }
    }

    mod test {
        #[test]
        fn trace_flags() {
            let flags = opentelemetry::trace::TraceFlags::SAMPLED;
            let flags2 = crate::wasi::observe::tracer::TraceFlags::from(flags);
            let flags3 = opentelemetry::trace::TraceFlags::from(flags2);
            assert_eq!(flags, flags3);
        }

        #[test]
        fn span_context() {
            let sc = opentelemetry::trace::SpanContext::new(
                opentelemetry::trace::TraceId::from_hex("4fb34cb4484029f7881399b149e41e98")
                    .unwrap(),
                opentelemetry::trace::SpanId::from_hex("9ffd58d3cd4dd90b").unwrap(),
                opentelemetry::trace::TraceFlags::SAMPLED,
                false,
                opentelemetry::trace::TraceState::from_key_value(vec![
                    ("foo", "bar"),
                    ("baz", "qux"),
                ])
                .unwrap(),
            );
            let sc2 = crate::wasi::observe::tracer::SpanContext::from(sc.clone());
            let sc3 = opentelemetry::trace::SpanContext::from(sc2);
            assert_eq!(sc, sc3);
        }
    }
}
