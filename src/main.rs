pub mod gpu_sort;
pub mod multicore_sort;
pub mod single_core_sort;

fn main() {
    let test_vec = vec![15, 53, 1, 24, 3, 1765, 22, 2, 8, 7, 4];
    //let sorted_vec = single_core_sort::merge_sort(&test_vec);
    //let sorted_vec = multicore_sort::merge_sort_parallel(&test_vec);
    //let sorted_vec = multicore_sort::merge_sort_threadpool(&test_vec, 8);
    let sorted_vec = multicore_sort::merge_sort_threadpool_chunks(&test_vec, 8);
    println!("Sorted vec: {sorted_vec:?}");
}
