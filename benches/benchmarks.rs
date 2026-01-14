use criterion::{criterion_group, criterion_main, Criterion};
use std::{hint::black_box, time::Duration};
use merge_sort::{
    multicore_sort::{
        merge_sort_parallel, merge_sort_parallel_limit, merge_sort_threadpool, merge_sort_threadpool_chunks
    },
    single_core_sort::merge_sort,
    gpu_sort::merge_sort_gpu,
};
const SIZE: usize = 1_000_000;

pub fn sequential_sort_benchmark(c: &mut Criterion) {
    let size = SIZE;
    let mut vec: Vec<i32> = Vec::with_capacity(size);
    for _ in 1..size {
        vec.push(rand::random());
    }
    c.bench_function("sequential sort {size}", |b| b.iter(|| merge_sort(black_box(&vec))));
}

pub fn parallel_sort_benchmark(c: &mut Criterion) {
    let size = SIZE;
    let mut vec: Vec<i32> = Vec::with_capacity(size);
    for _ in 1..size {
        vec.push(rand::random());
    }
    c.bench_function("parallel sort {size}", |b| b.iter(|| merge_sort_parallel(black_box(&vec))));
}

pub fn threadpool_sort_benchmark(c: &mut Criterion) {
    let size = SIZE;
    let mut vec: Vec<i32> = Vec::with_capacity(size);
    for _ in 1..size {
        vec.push(rand::random());
    }
    c.bench_function("threadpool sort {size}", |b| b.iter(|| merge_sort_threadpool(black_box(&vec), 8)));
}

pub fn parallel_limit_sort_benchmark(c: &mut Criterion) {
    let size = SIZE;
    let mut vec: Vec<i32> = Vec::with_capacity(size);
    for _ in 1..size {
        vec.push(rand::random());
    }
    c.bench_function("parallel limit sort {size}", |b| b.iter(|| merge_sort_parallel_limit(black_box(&vec), 8)));
}

pub fn threadpool_chunks_sort_benchmark(c: &mut Criterion) {
    let size = SIZE;
    let mut vec: Vec<i32> = Vec::with_capacity(size);
    for _ in 1..size {
        vec.push(rand::random());
    }
    c.bench_function("threadpool sort in chunks {size}", |b| b.iter(|| merge_sort_threadpool_chunks(black_box(&vec), 8)));
}

pub fn gpu_sort_benchmark(c: &mut Criterion) {
    let size = SIZE;
    let mut vec: Vec<i32> = Vec::with_capacity(size);
    for _ in 1..size {
        vec.push(rand::random());
    }
    c.bench_function("gpu merge sort {size}", |b| b.iter(|| merge_sort_gpu(black_box(vec.clone()))));
}

criterion_group!(
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(20)).sample_size(50);
    targets =
        sequential_sort_benchmark,
        threadpool_sort_benchmark,
        parallel_limit_sort_benchmark,
        threadpool_chunks_sort_benchmark,
        gpu_sort_benchmark,
);
criterion_main!(benches);
