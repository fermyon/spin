#![allow(missing_docs)]

wasmtime::component::bindgen!({
    inline: r#"
    package fermyon:runtime;
    world host {
        include fermyon:spin/host;
        include fermyon:spin/platform@2.0.0;
    }
    "#,
    path: "../../wit",
    async: true,
    // The following is a roundabout way of saying "the host implementations for these interfaces don't trap"
    trappable_error_type: {
        "fermyon:spin/config/error" => v1::config::Error,
        "fermyon:spin/http-types/http-error" => v1::http_types::HttpError,
        "fermyon:spin/llm@2.0.0/error" => v2::llm::Error,
        "fermyon:spin/llm/error" => v1::llm::Error,
        "fermyon:spin/mqtt@2.0.0/error" => v2::mqtt::Error,
        "fermyon:spin/mysql/mysql-error" => v1::mysql::MysqlError,
        "fermyon:spin/postgres/pg-error" => v1::postgres::PgError,
        "fermyon:spin/rdbms-types@2.0.0/error" => v2::rdbms_types::Error,
        "fermyon:spin/redis-types/error" => v1::redis_types::Error,
        "fermyon:spin/redis@2.0.0/error" => v2::redis::Error,
        "fermyon:spin/sqlite@2.0.0/error" => v2::sqlite::Error,
        "fermyon:spin/sqlite/error" => v1::sqlite::Error,
        "fermyon:spin/variables@2.0.0/error" => v2::variables::Error,
    },
    trappable_imports: true,
});

pub use fermyon::spin as v1;
pub use fermyon::spin2_0_0 as v2;

mod conversions;
