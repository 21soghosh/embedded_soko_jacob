use criterion::{criterion_group, criterion_main, Criterion};
use rusttracer::render_dev;

<<<<<<< HEAD
fn small_sample() -> Criterion {
    Criterion::default().sample_size(100)
}

=======
>>>>>>> mutex+buffer
fn bench_render_small(c: &mut Criterion) {
    c.bench_function("render_dev_scene", |b| {
        b.iter(|| {
            render_dev();
        });
    });
}

<<<<<<< HEAD
criterion_group! {
    name = benches;
    config = small_sample();
    targets = bench_render_small
}
=======
criterion_group!(benches, bench_render_small);
>>>>>>> mutex+buffer
criterion_main!(benches);
