use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::Arc;

use criterion::{criterion_group, criterion_main, Criterion};

use http::uri::Scheme;
use http::Request;
use spin_testing::{assert_http_response_success, HttpTestConfig};
use spin_trigger_http::HttpTrigger;
use tokio::runtime::Runtime;

criterion_main!(benches);
criterion_group!(
    benches,
    bench_startup,
    bench_spin_concurrency_minimal,
    bench_wagi_concurrency_minimal,
);

async fn spin_trigger() -> Arc<HttpTrigger> {
    Arc::new(
        HttpTestConfig::default()
            .test_program("spin-http-benchmark.wasm")
            .http_spin_trigger("/")
            .build_trigger()
            .await,
    )
}

async fn wagi_trigger() -> Arc<HttpTrigger> {
    Arc::new(
        HttpTestConfig::default()
            .test_program("wagi-benchmark.wasm")
            .http_wagi_trigger("/", Default::default())
            .build_trigger()
            .await,
    )
}

// Benchmark time to start and process one request
fn bench_startup(c: &mut Criterion) {
    let async_runtime = Runtime::new().unwrap();

    let mut group = c.benchmark_group("startup");
    group.bench_function("spin-executor", |b| {
        b.to_async(&async_runtime).iter(|| async {
            let trigger = spin_trigger().await;
            run(&trigger, "/").await;
        });
    });
    group.bench_function("spin-wagi-executor", |b| {
        b.to_async(&async_runtime).iter(|| async {
            let trigger = wagi_trigger().await;
            run(&trigger, "/").await;
        });
    });
}

fn bench_spin_concurrency_minimal(c: &mut Criterion) {
    bench_concurrency_minimal(c, "spin-executor", spin_trigger);
}
fn bench_wagi_concurrency_minimal(c: &mut Criterion) {
    bench_concurrency_minimal(c, "spin-wagi-executor", wagi_trigger);
}

fn bench_concurrency_minimal<F: Future<Output = Arc<HttpTrigger>>>(
    c: &mut Criterion,
    name: &str,
    mk: fn() -> F,
) {
    let async_runtime = Runtime::new().unwrap();
    let trigger = async_runtime.block_on(mk());

    for task in ["/?sleep=1", "/?noop", "/?cpu=1"] {
        let mut group = c.benchmark_group(format!("{name}{task}"));
        for concurrency in concurrency_steps() {
            group.bench_function(format!("concurrency-{}", concurrency), |b| {
                let done = Arc::new(AtomicBool::new(false));
                let background = (0..concurrency - 1)
                    .map(|_| {
                        let trigger = trigger.clone();
                        let done = done.clone();
                        async_runtime.spawn(async move {
                            while !done.load(Relaxed) {
                                run(&trigger, task).await;
                            }
                        })
                    })
                    .collect::<Vec<_>>();
                b.to_async(&async_runtime).iter(|| run(&trigger, task));
                done.store(true, Relaxed);
                for task in background {
                    async_runtime.block_on(task).unwrap();
                }
            });
        }
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

async fn run(trigger: &HttpTrigger, path: &str) {
    let req = Request::get(path.to_string())
        .body(Default::default())
        .unwrap();
    let resp = trigger
        .handle(req, Scheme::HTTP, "127.0.0.1:55555".parse().unwrap())
        .await
        .unwrap();
    assert_http_response_success(&resp);
}
