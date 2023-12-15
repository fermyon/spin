#[cfg(feature = "e2e-tests")]
mod runtime_tests {
    use runtime_tests::Config;
    use std::path::PathBuf;

    // TODO: write a proc macro that reads from the tests folder
    // and creates tests for every subdirectory
    macro_rules! test {
        ($ident:ident, $path:literal) => {
            #[test]
            fn $ident() {
                run($path)
            }
        };
    }

    test!(outbound_mysql, "outbound-mysql");
    test!(outbound_mysql_no_permission, "outbound-mysql-no-permission");
    test!(outbound_postgres, "outbound-postgres");
    test!(
        outbound_postgres_no_permission,
        "outbound-postgres-no-permission"
    );
    test!(outbound_redis, "outbound-redis");
    test!(outbound_redis_no_permission, "outbound-redis-no-permission");
    test!(sqlite, "sqlite");
    test!(sqlite_no_permission, "sqlite-no-permission");
    test!(key_value, "key-value");
    test!(key_value_no_permission, "key-value-no-permission");
    test!(variables, "variables");
    test!(tcp_sockets, "tcp-sockets");
    test!(tcp_sockets_ip_range, "tcp-sockets-ip-range");
    test!(
        tcp_sockets_no_port_permission,
        "tcp-sockets-no-port-permission"
    );
    test!(tcp_sockets_no_ip_permission, "tcp-sockets-no-ip-permission");

    fn run(name: &str) {
        let spin_binary_path = env!("CARGO_BIN_EXE_spin").into();
        let tests_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/runtime-tests/tests");
        let config = Config {
            spin_binary_path,
            tests_path,
            on_error: runtime_tests::OnTestError::Panic,
        };
        let path = config.tests_path.join(name);
        runtime_tests::bootstrap_and_run(&path, &config)
            .expect("failed to bootstrap runtime tests tests");
    }
}
