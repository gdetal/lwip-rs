use criterion::*;

mod common;

fn benchmark(c: &mut Criterion) {
    common::benchmark_futures(common::benchmark_async(c, "ipv4", "127.0.0.1"));
}

criterion_group! {
    name = benches;
    config = common::criterion();
    targets = benchmark,
}
criterion_main!(benches);
