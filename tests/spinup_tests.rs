#[cfg(feature = "e2e-tests")]
mod testcases;

#[cfg(feature = "e2e-tests")]
mod spinup_tests {
    use super::testcases;
    use {e2e_testing::controller::Controller, e2e_testing::spin_controller::SpinUp};
    const CONTROLLER: &dyn Controller = &SpinUp {};

    #[tokio::test]
    async fn component_outbound_http_works() {
        testcases::component_outbound_http_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn application_variables_default_works() {
        testcases::application_variables_default_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn key_value_works() {
        testcases::key_value_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_go_works() {
        testcases::http_go_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_c_works() {
        testcases::http_c_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_rust_works() {
        testcases::http_rust_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_zig_works() {
        testcases::http_zig_works(CONTROLLER).await
    }

    #[tokio::test]
    #[cfg(target_arch = "x86_64")]
    async fn http_grain_works() {
        testcases::http_grain_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_ts_works() {
        testcases::http_ts_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_js_works() {
        testcases::http_js_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_python_works() {
        testcases::http_python_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_php_works() {
        testcases::http_php_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_swift_works() {
        testcases::http_swift_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn assets_routing_works() {
        testcases::assets_routing_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn head_rust_sdk_http() {
        testcases::head_rust_sdk_http(CONTROLLER).await
    }

    #[tokio::test]
    async fn head_rust_sdk_redis() {
        testcases::head_rust_sdk_redis(CONTROLLER).await
    }

    #[tokio::test]
    async fn llm_works() {
        testcases::llm_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn header_env_routes_works() {
        testcases::header_env_routes_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn header_dynamic_env_works() {
        testcases::header_dynamic_env_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_rust_outbound_mysql_works() {
        testcases::http_rust_outbound_mysql_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_rust_outbound_pg_works() {
        testcases::http_rust_outbound_pg_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn redis_go_works() {
        testcases::redis_go_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn redis_rust_works() {
        testcases::redis_rust_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn registry_works() {
        testcases::registry_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn longevity_apps_works() {
        testcases::longevity_apps_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn error_messages() {
        testcases::error_messages(CONTROLLER).await
    }
}
