use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use merge_sort::{
    single_core_sort::merge_sort,
    multicore_sort::{
        merge_sort_parallel,
        merge_sort_threadpool,
    },
};

pub fn sequential_sort_benchmark(c: &mut Criterion) {
    let size = 100000;
    let mut vec: Vec<i32> = Vec::with_capacity(size);
    for _ in 1..size {
        vec.push(rand::random());
    }
    c.bench_function("sequential sort {size}", |b| b.iter(|| merge_sort(black_box(&vec))));
}

pub fn parallel_sort_benchmark(c: &mut Criterion) {
    let size = 100000;
    let mut vec: Vec<i32> = Vec::with_capacity(size);
    for _ in 1..size {
        vec.push(rand::random());
    }
    c.bench_function("parallel sort {size}", |b| b.iter(|| merge_sort_parallel(black_box(&vec))));
}

pub fn threadpool_sort_benchmark(c: &mut Criterion) {
    let size = 100000;
    let mut vec: Vec<i32> = Vec::with_capacity(size);
    for _ in 1..size {
        vec.push(rand::random());
    }
    c.bench_function("threadpool sort {size}", |b| b.iter(|| merge_sort_threadpool(black_box(&vec), 8)));
}

criterion_group!(benches, sequential_sort_benchmark, threadpool_sort_benchmark, parallel_sort_benchmark);
criterion_main!(benches);

