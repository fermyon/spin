use std::path::PathBuf;
use std::time::Instant;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use futures::future::join_all;
use http::{Request, StatusCode};
use spin_config::{
    ApplicationInformation, ApplicationOrigin, Configuration, CoreComponent, HttpConfig,
    HttpExecutor, ModuleSource, SpinVersion, TriggerConfig,
};
use spin_http_engine::HttpTrigger;
use tokio::runtime::Runtime;
use tokio::task;

criterion_main!(benches);
criterion_group!(
    benches,
    bench_startup,
    bench_spin_concurrency_minimal,
    bench_wagi_concurrency_minimal,
);

const SPIN_HTTP_MINIMAL_PATH: &str = "../../target/test-programs/spin-http-benchmark.wasm";

const WAGI_MINIMAL_PATH: &str = "../../target/test-programs/wagi-benchmark.wasm";

// Benchmark time to start and process one request
fn bench_startup(c: &mut Criterion) {
    let async_runtime = Runtime::new().unwrap();

    let mut group = c.benchmark_group("startup");
    group.bench_function("spin-executor", |b| {
        b.to_async(&async_runtime).iter(|| async {
            let trigger = build_trigger(SPIN_HTTP_MINIMAL_PATH, HttpExecutor::Spin).await;
            run_concurrent_requests(&trigger, 0, 1).await;
        });
    });
    group.bench_function("spin-wagi-executor", |b| {
        b.to_async(&async_runtime).iter(|| async {
            let trigger =
                build_trigger(WAGI_MINIMAL_PATH, HttpExecutor::Wagi(Default::default())).await;
            run_concurrent_requests(&trigger, 0, 1).await;
        });
    });
}

// Benchmark SpinHttpExecutor time to process requests at various levels of concurrency
fn bench_spin_concurrency_minimal(c: &mut Criterion) {
    let async_runtime = Runtime::new().unwrap();

    let spin_trigger =
        async_runtime.block_on(build_trigger(SPIN_HTTP_MINIMAL_PATH, HttpExecutor::Spin));

    let sleep_ms = 1;
    let mut group = c.benchmark_group(format!("spin-executor/sleep-{}ms", sleep_ms));
    for concurrency in concurrency_steps() {
        let bench_inner = || {
            black_box(run_concurrent_requests(
                &spin_trigger,
                sleep_ms,
                concurrency,
            ))
        };

        group.bench_function(format!("concurrency-{}", concurrency), |b| {
            b.to_async(&async_runtime).iter_custom(|iters| async move {
                let start = Instant::now();
                for _ in 0..iters {
                    bench_inner().await;
                }
                start.elapsed()
            });
        });
    }
}

// Benchmark WagiHttpExecutor time to process requests at various levels of concurrency
fn bench_wagi_concurrency_minimal(c: &mut Criterion) {
    let async_runtime = Runtime::new().unwrap();

    let wagi_trigger = async_runtime.block_on(build_trigger(
        WAGI_MINIMAL_PATH,
        HttpExecutor::Wagi(Default::default()),
    ));

    let sleep_ms = 1;
    let mut group = c.benchmark_group(format!("spin-wagi-executor/sleep-{}ms", sleep_ms));
    for concurrency in concurrency_steps() {
        let bench_inner = || {
            black_box(run_concurrent_requests(
                &wagi_trigger,
                sleep_ms,
                concurrency,
            ))
        };

        group.bench_function(format!("concurrency-{}", concurrency), |b| {
            b.to_async(&async_runtime).iter_custom(|iters| async move {
                let start = Instant::now();
                for _ in 0..iters {
                    bench_inner().await;
                }
                start.elapsed()
            });
        });
    }
}

// Helpers

fn concurrency_steps() -> [u32; 3] {
    let cpus = num_cpus::get() as u32;
    if cpus > 1 {
        [1, cpus, cpus * 4]
    } else {
        [1, 2, 4]
    }
}

async fn run_concurrent_requests(trigger: &HttpTrigger, sleep_ms: u32, concurrency: u32) {
    join_all((0..concurrency).map(|_| {
        let trigger = trigger.to_owned();
        task::spawn(async move {
            let req = Request::get(format!("/?sleep={}", sleep_ms))
                .body(Default::default())
                .unwrap();
            let resp = trigger
                .handle(req, "127.0.0.1:55555".parse().unwrap())
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
        })
    }))
    .await;
}

async fn build_trigger(entrypoint_path: &str, executor: HttpExecutor) -> HttpTrigger {
    let entrypoint_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(entrypoint_path);

    let info = ApplicationInformation {
        spin_version: SpinVersion::V1,
        name: "bench-app".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        authors: vec![],
        trigger: spin_config::ApplicationTrigger::Http(spin_config::HttpTriggerConfiguration {
            base: "/".to_owned(),
        }),
        namespace: None,
        origin: ApplicationOrigin::File("bench_spin.toml".into()),
    };

    let component = CoreComponent {
        source: ModuleSource::FileReference(entrypoint_path),
        id: "bench".to_string(),
        trigger: TriggerConfig::Http(HttpConfig {
            route: "/".to_string(),
            executor: Some(executor),
        }),
        wasm: Default::default(),
    };
    let components = vec![component];

    let cfg = Configuration::<CoreComponent> { info, components };
    HttpTrigger::new("".to_string(), cfg, None, None, None)
        .await
        .unwrap()
}
