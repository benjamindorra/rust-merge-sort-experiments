use std::sync::{Arc, Mutex, mpsc};
use std::thread;

mod threadpool;
use crate::multicore_sort::threadpool::ThreadPool;
// Trait aliasing for readibility
// https://stackoverflow.com/questions/26070559/is-there-any-way-to-create-a-type-alias-for-multiple-traits
pub trait SortTraits: Clone + PartialOrd + Send + Sync + 'static {}
impl<T: Clone + PartialOrd + Send + Sync + 'static> SortTraits for T {}

struct SortVecPair<T: SortTraits> {
    bin_size: Mutex<usize>,
    length: usize,
    values: Vec<Mutex<T>>,
    buffer: Vec<Mutex<T>>,
}
struct BinsPositions {
    start: usize,
    mid: usize,
    end: usize,
}
struct SortThreadData<T: SortTraits> {
    vec_pair: Arc<SortVecPair<T>>,
    bins_positions: BinsPositions,
}
impl<T: SortTraits> SortVecPair<T> {
    fn new(unsorted_vec: &[T]) -> SortVecPair<T> {
        let mut buffer: Vec<Mutex<T>> = Vec::with_capacity(unsorted_vec.len());
        let mut values: Vec<Mutex<T>> = Vec::with_capacity(unsorted_vec.len());
        for val in unsorted_vec {
            buffer.push(Mutex::new(val.clone()));
            values.push(Mutex::new(val.clone()));
        }
        SortVecPair {
            bin_size: Mutex::new(1),
            length: unsorted_vec.len(),
            values: values,
            buffer: buffer,
        }
    }

    fn finish_merge(&self) {
        // Double the bin size to prepare for the next merging iteration
        let mut bin_size = self
            .bin_size
            .lock()
            .expect("Could not lock the bin size mutex");
        *bin_size *= 2;
    }
    fn get_bin_size(&self) -> usize {
        let bin_size = self
            .bin_size
            .lock()
            .expect("Could not lock the bin size mutex");
        *bin_size
    }

    fn get_values(&self) -> Vec<T> {
        let mut values: Vec<T> = Vec::with_capacity(self.length);
        for val in &self.values {
            values.push(val.lock().expect("Could not acquire a value mutex").clone());
        }
        values
    }
    fn get_bins_positions(vec_pair: Arc<SortVecPair<T>>, id: usize) -> Option<SortThreadData<T>> {
        let bin_size = *vec_pair
            .bin_size
            .lock()
            .expect("could not lock the bin size mutex");
        let start = id * 2 * bin_size;
        let mid = start + bin_size;
        let end = mid + bin_size;
        if end < vec_pair.length {
            let bins_positions = BinsPositions { start, mid, end };
            Some(SortThreadData {
                vec_pair: Arc::clone(&vec_pair),
                bins_positions,
            })
        } else if mid < vec_pair.length {
            let bins_positions = BinsPositions {
                start,
                mid,
                end: vec_pair.length.into(),
            };
            Some(SortThreadData {
                vec_pair: Arc::clone(&vec_pair),
                bins_positions,
            })
        } else {
            None
        }
    }
}

pub fn merge_sort_parallel<T: SortTraits>(input: &[T]) -> Vec<T> {
    let sort_vec_pair = SortVecPair::new(input);
    let sort_vec_pair = Arc::new(sort_vec_pair);
    while sort_vec_pair.get_bin_size() < input.len() {
        let mut handles_vec = Vec::new();
        let mut id = 0;
        while let Some(sort_thread_data) =
            SortVecPair::get_bins_positions(Arc::clone(&sort_vec_pair), id)
        {
            let handle = thread::spawn(move || merge_bins(sort_thread_data));
            handles_vec.push(handle);
            id += 1;
        }
        for handle in handles_vec {
            _ = handle.join();
        }
        // Copy buffer into values vector, increase bin size
        sort_vec_pair.finish_merge();
    }
    sort_vec_pair.get_values()
}

// Attempt to simplify the threadpool approach to improve performance,
// at the cost of reinstancing the threads at each iteration
// Goal: go past the single threaded performance
pub fn merge_sort_parallel_limit<T: SortTraits>(input: &[T], threads: usize) -> Vec<T> {
    let sort_vec_pair = SortVecPair::new(input);
    let sort_vec_pair = Arc::new(sort_vec_pair);
    let input_length = input.len();
    while sort_vec_pair.get_bin_size() < input_length {
        let bin_size = sort_vec_pair.get_bin_size();
        let mut handles_vec = Vec::new();
        let max_ops = input_length.div_ceil(threads * 2 * bin_size);
        for ct in 0..threads {
            let sort_vec_pair = Arc::clone(&sort_vec_pair);
            let handle = thread::spawn(move || {
                let start = ct * max_ops;
                let limit = start + max_ops;
                for id in start..limit {
                    match SortVecPair::get_bins_positions(Arc::clone(&sort_vec_pair), id) {
                        Some(sort_thread_data) => {
                            merge_bins(sort_thread_data);
                        }
                        None => break,
                    };
                }
            });
            handles_vec.push(handle);
        }
        for handle in handles_vec {
            _ = handle.join();
        }
        // Copy buffer into values vector, increase bin size
        sort_vec_pair.finish_merge();
    }
    sort_vec_pair.get_values()
}

pub fn merge_sort_threadpool<T: SortTraits>(input: &[T], threads: usize) -> Vec<T> {
    let sort_vec_pair = SortVecPair::new(input);
    let sort_vec_pair = Arc::new(sort_vec_pair);
    let threadpool = ThreadPool::new(threads);
    while sort_vec_pair.get_bin_size() < input.len() {
        // Channel to keep track of the pool progress through the tasks
        let (task_progress_write, task_progress_read) = mpsc::channel();
        let task_progress_write = Arc::new(task_progress_write);
        let mut num_tasks = 0;
        while let Some(sort_thread_data) =
            SortVecPair::get_bins_positions(Arc::clone(&sort_vec_pair), num_tasks)
        {
            let task_progress_write = Arc::clone(&task_progress_write);
            threadpool.execute(move || {
                merge_bins(sort_thread_data);
                task_progress_write.send(()).unwrap();
            });
            num_tasks += 1;
        }
        // Wait until the tasks finish
        for _ in 0..num_tasks {
            task_progress_read.recv().unwrap();
        }
        // Copy buffer into values vector, increase bin size
        sort_vec_pair.finish_merge();
    }
    sort_vec_pair.get_values()
}

// Attempt to speed up the parallel processing by splitting the code into bigger tasks
pub fn merge_sort_threadpool_chunks<T: SortTraits>(input: &[T], threads: usize) -> Vec<T> {
    let sort_vec_pair = SortVecPair::new(input);
    let sort_vec_pair = Arc::new(sort_vec_pair);
    let threadpool = ThreadPool::new(threads);
    let input_len = input.len();
    while sort_vec_pair.get_bin_size() < input_len {
        // Channel to keep track of the pool progress through the tasks
        let (task_progress_write, task_progress_read) = mpsc::channel();
        let task_progress_write = Arc::new(task_progress_write);
        let mut num_tasks = 0;
        let bin_size = sort_vec_pair.get_bin_size();
        let num_ops_per_thread = input_len.div_ceil(2 * bin_size * threads);
        for ct in 0..threads {
            let sort_vec_pair = Arc::clone(&sort_vec_pair);
            let task_progress_write = Arc::clone(&task_progress_write);
            threadpool.execute(move || {
                let mut id = ct * num_ops_per_thread;
                while id < (ct + 1) * num_ops_per_thread {
                    if let Some(sort_thread_data) =
                        SortVecPair::get_bins_positions(Arc::clone(&sort_vec_pair), id)
                    {
                        merge_bins(sort_thread_data);
                    };
                    id += 1;
                }
                task_progress_write.send(()).unwrap();
            });
            num_tasks += 1;
        }
        // Wait until the tasks finish
        for _ in 0..num_tasks {
            task_progress_read.recv().unwrap();
        }
        // Copy buffer into values vector, increase bin size
        sort_vec_pair.finish_merge();
    }
    sort_vec_pair.get_values()
}

fn merge_bins<T: SortTraits>(sort_thread_data: SortThreadData<T>) {
    let SortThreadData {
        vec_pair,
        bins_positions,
    } = sort_thread_data;
    let bin1 = &vec_pair.values[bins_positions.start..bins_positions.mid];
    let bin2 = &vec_pair.values[bins_positions.mid..bins_positions.end];
    let buf = &vec_pair.buffer[bins_positions.start..bins_positions.end];
    let mut id1 = 0;
    let mut id2 = 0;
    for min_val in buf {
        let mut min_val = min_val.lock().unwrap();
        if id1 >= bin1.len() {
            *min_val = bin2[id2].lock().unwrap().clone().into();
            id2 += 1;
        } else if id2 >= bin2.len() {
            *min_val = bin1[id1].lock().unwrap().clone().into();
            id1 += 1;
        } else {
            let val1 = bin1[id1].lock().unwrap().clone();
            let val2 = bin2[id2].lock().unwrap().clone();
            if val1 <= val2 {
                *min_val = val1.into();
                id1 += 1;
            } else {
                *min_val = val2.into();
                id2 += 1;
            }
        }
    }
    // Reinject values in the sorted section of the values in parallel
    // should be faster than doing it all on one thread at the end
    let vals = &vec_pair.values[bins_positions.start..bins_positions.end];
    for (v, b) in std::iter::zip(vals, buf) {
        let mut vl = v
            .lock()
            .expect("Value within a sorted bin should be accessible and lockable");
        let bl = b
            .lock()
            .expect("Buffer values within a sorted bin should be accessible and lockable");
        *vl = bl.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_small_vec_parallel() {
        let test_vec = vec![15, 53, 1, 24, 25, 3];
        assert_eq!(merge_sort_parallel(&test_vec), vec![1, 3, 15, 24, 25, 53]);
    }

    #[test]
    fn sort_small_vec_threadpool() {
        let test_vec = vec![15, 53, 1, 24, 25, 3];
        assert_eq!(
            merge_sort_threadpool(&test_vec, 8),
            vec![1, 3, 15, 24, 25, 53]
        );
    }

    #[test]
    fn sort_small_vec_parallel_limit() {
        let test_vec = vec![15, 53, 1, 24, 25, 3, 37, 12, 56];
        assert_eq!(
            merge_sort_parallel_limit(&test_vec, 8),
            vec![1, 3, 12, 15, 24, 25, 37, 53, 56]
        );
    }

    #[test]
    fn sort_small_vec_threadpool_chunks() {
        let test_vec = vec![15, 53, 1, 24, 25, 3, 37, 12, 56];
        assert_eq!(
            merge_sort_threadpool_chunks(&test_vec, 8),
            vec![1, 3, 12, 15, 24, 25, 37, 53, 56]
        );
    }
}
