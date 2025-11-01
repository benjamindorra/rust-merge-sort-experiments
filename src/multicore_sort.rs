use std::sync::atomic::{AtomicUsize, Ordering};
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
        // Reinject the buffer in the values
        for i in 0..self.length {
            let mut val = self.values[i].lock().expect("Mutex lock error");
            let buf = self.buffer[i].lock().expect("Mutex lock error");
            *val = buf.clone();
        }
        // Double the bin size to prepare for the next merging iteration
        let mut bin_size = self.bin_size.lock().expect("Could not lock the bin size mutex");
        *bin_size *= 2;
    }
    fn get_bin_size(&self) -> usize {
        let bin_size = self.bin_size.lock().expect("Could not lock the bin size mutex");
        *bin_size
    }

    fn get_values(&self) -> Vec<T> {
        let mut values: Vec<T> = Vec::with_capacity(self.length);
        for val in &self.values {
            values.push(val.lock().expect("Could not acquire a value mutex").clone());
        }
        values
    }
    fn get_bins_positions(
        vec_pair: Arc<SortVecPair<T>>,
        end_prev: usize,
    ) -> Option<SortThreadData<T>> {
        let bin_size = vec_pair.bin_size.lock().expect("could not lock the bin size mutex");
        if end_prev + 2 * *bin_size < vec_pair.length {
            let bins_positions = BinsPositions {
                start: end_prev,
                mid: end_prev + *bin_size,
                end: (end_prev + 2 * *bin_size).into(),
            };
            Some(SortThreadData {
                vec_pair: Arc::clone(&vec_pair),
                bins_positions,
            })
        } else if end_prev + *bin_size < vec_pair.length {
            let bins_positions = BinsPositions {
                start: end_prev,
                mid: end_prev + *bin_size,
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
        let mut end_prev = 0;
        while let Some(sort_thread_data) = SortVecPair::get_bins_positions(Arc::clone(&sort_vec_pair), end_prev.into())
        {
            end_prev = sort_thread_data.bins_positions.end.clone();
            let handle = thread::spawn(move || merge_bins(sort_thread_data));
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

// Attempt to simplify the threadpool approach to improve performance,
// at the cost of reinstancing the threads at each iteration
// Goal: go past the single threaded performance
pub fn merge_sort_parallel_limit<T: SortTraits>(input: &[T], threads: usize) -> Vec<T> {
    let sort_vec_pair = SortVecPair::new(input);
    let sort_vec_pair = Arc::new(sort_vec_pair);
    let input_length = input.len();
    while sort_vec_pair.get_bin_size() < input_length {  
        let end_prev = Arc::new(AtomicUsize::new(0));
        let mut handles_vec = Vec::new();
        for ct in 0..threads {
            let sort_vec_pair = Arc::clone(&sort_vec_pair);
            let end_prev = Arc::clone(&end_prev);
            let handle = thread::spawn(move || {
                let limit = if ct<(threads-1) {input_length/threads} else {input_length-end_prev.load(Ordering::SeqCst)};
                for _ in 0..limit {
                    match SortVecPair::get_bins_positions(Arc::clone(&sort_vec_pair), end_prev.load(Ordering::SeqCst)) {
                        Some(sort_thread_data) => {
                            end_prev.store(sort_thread_data.bins_positions.end.clone(), Ordering::SeqCst);
                            merge_bins(sort_thread_data);
                        }
                        None => break
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
        let mut end_prev = 0;
        while let Some(sort_thread_data) = SortVecPair::get_bins_positions(Arc::clone(&sort_vec_pair), end_prev)
        {
            end_prev = sort_thread_data.bins_positions.end;
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
}
