mod testcases;
#[cfg(feature = "new-e2e-tests")]
use {e2e_testing::controller::Controller, e2e_testing::spin_controller::SpinUp};

#[cfg(feature = "new-e2e-tests")]
const CONTROLLER: &dyn Controller = &SpinUp {};

#[cfg(feature = "new-e2e-tests")]
mod spinup_tests {
    use super::testcases;
    use super::CONTROLLER;

    #[tokio::test]
    async fn http_go_works() {
        testcases::all::http_go_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_c_works() {
        testcases::all::http_c_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_rust_works() {
        testcases::all::http_rust_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_zig_works() {
        testcases::all::http_zig_works(CONTROLLER).await
    }

    #[tokio::test]
    #[cfg(target_arch = "x86_64")]
    async fn http_grain_works() {
        testcases::all::http_grain_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_ts_works() {
        testcases::all::http_ts_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_js_works() {
        testcases::all::http_js_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn assets_routing_works() {
        testcases::all::assets_routing_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn simple_spin_rust_works() {
        testcases::all::simple_spin_rust_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn header_env_routes_works() {
        testcases::all::header_env_routes_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn header_dynamic_env_works() {
        testcases::all::header_dynamic_env_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_rust_outbound_mysql_works() {
        testcases::all::http_rust_outbound_mysql_works(CONTROLLER).await
    }
}
