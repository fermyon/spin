use std::sync::Arc;
use std::time::Instant;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use futures::future::join_all;
use http::uri::Scheme;
use http::Request;
use spin_testing::{assert_http_response_success, HttpTestConfig};
use spin_trigger_http::HttpTrigger;
use tokio::runtime::Runtime;
use tokio::task;

criterion_main!(benches);
criterion_group!(
    benches,
    bench_startup,
    bench_spin_concurrency_minimal,
    bench_wagi_concurrency_minimal,
);

// Benchmark time to start and process one request
fn bench_startup(c: &mut Criterion) {
    let async_runtime = Runtime::new().unwrap();

    let mut group = c.benchmark_group("startup");
    group.bench_function("spin-executor", |b| {
        b.to_async(&async_runtime).iter(|| async {
            let trigger = HttpTestConfig::default()
                .test_program("spin-http-benchmark.wasm")
                .http_spin_trigger("/")
                .build_trigger()
                .await;
            run_concurrent_requests(Arc::new(trigger), 0, 1).await;
        });
    });
    group.bench_function("spin-wagi-executor", |b| {
        b.to_async(&async_runtime).iter(|| async {
            let trigger = HttpTestConfig::default()
                .test_program("wagi-benchmark.wasm")
                .http_wagi_trigger("/", Default::default())
                .build_trigger()
                .await;
            run_concurrent_requests(Arc::new(trigger), 0, 1).await;
        });
    });
}

// Benchmark SpinHttpExecutor time to process requests at various levels of concurrency
fn bench_spin_concurrency_minimal(c: &mut Criterion) {
    let async_runtime = Runtime::new().unwrap();

    let spin_trigger: Arc<HttpTrigger> = Arc::new(
        async_runtime.block_on(
            HttpTestConfig::default()
                .test_program("spin-http-benchmark.wasm")
                .http_spin_trigger("/")
                .build_trigger(),
        ),
    );

    let sleep_ms = 1;
    let mut group = c.benchmark_group(format!("spin-executor/sleep-{}ms", sleep_ms));
    for concurrency in concurrency_steps() {
        let bench_inner = || {
            black_box(run_concurrent_requests(
                spin_trigger.clone(),
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

    let wagi_trigger: Arc<HttpTrigger> = Arc::new(
        async_runtime.block_on(
            HttpTestConfig::default()
                .test_program("wagi-benchmark.wasm")
                .http_wagi_trigger("/", Default::default())
                .build_trigger(),
        ),
    );

    let sleep_ms = 1;
    let mut group = c.benchmark_group(format!("spin-wagi-executor/sleep-{}ms", sleep_ms));
    for concurrency in concurrency_steps() {
        let bench_inner = || {
            black_box(run_concurrent_requests(
                wagi_trigger.clone(),
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

async fn run_concurrent_requests(trigger: Arc<HttpTrigger>, sleep_ms: u32, concurrency: u32) {
    join_all((0..concurrency).map(|_| {
        let trigger = trigger.clone();
        task::spawn(async move {
            let req = Request::get(format!("/?sleep={}", sleep_ms))
                .body(Default::default())
                .unwrap();
            let resp = trigger
                .handle(req, Scheme::HTTP, "127.0.0.1:55555".parse().unwrap())
                .await
                .unwrap();
            assert_http_response_success(&resp);
        })
    }))
    .await;
}
