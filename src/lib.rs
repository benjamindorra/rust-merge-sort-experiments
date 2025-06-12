use std::cmp::Ord;
// Trait aliasing for readibility
// https://stackoverflow.com/questions/26070559/is-there-any-way-to-create-a-type-alias-for-multiple-traits
pub trait SortTraits: Clone + Ord {}
impl<T: Clone + Ord> SortTraits for T {} 

pub struct SortVecPair<T: SortTraits> {
    bin_size: usize,
    length: usize,
    values: Vec<T>,
    buffer: Vec<T>,
}

pub struct SortVecPairIterMut<'a, T: SortTraits> {
    vec_pair: &'a mut SortVecPair<T>,
    index: usize,
}

use std::iter::Iterator;

impl<'a, T: SortTraits> IntoIterator for &'a mut SortVecPair<T> {
    type Item = (&'a [T], &'a [T], &'a mut [T]);
    type IntoIter = SortVecPairIterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        SortVecPairIterMut {
            vec_pair: self,
            index: 0,
        }
    }
}

impl<'a, T: SortTraits> Iterator for SortVecPairIterMut<'a, T> {
    type Item = (&'a [T], &'a [T], &'a mut [T]);

    fn next(&mut self) -> Option<Self::Item> {
        let bin_size = self.vec_pair.bin_size;
        if self.index + 2 * bin_size < self.vec_pair.length {
            let bins = self.get_bins(self.index, self.index + bin_size, self.index + 2 * bin_size);
            self.index += 2 * bin_size;
            Some(bins)
        } else {
            None
        }
    }
}

impl<T: SortTraits> SortVecPair<T> {
    fn new(unsorted_vec: &[T]) -> SortVecPair<T> {
        SortVecPair {
            bin_size: 1,
            length: unsorted_vec.len(),
            values: unsorted_vec.to_vec(),
            buffer: unsorted_vec.to_vec(),
        }
    }

    fn finish_merge(&mut self) {
        // Merge the last bin if the array is not divisible
        // into pairs of bins
        // This is separate as this part cannot be parallelized
        if let Some((bin1, bin2, buf)) = self.get_last_bins() {
            merge_bins(bin1, bin2, buf);
        }
        // Reinject the buffer in the values
        self.values = self.buffer.clone();
        // Double the bin size to prepare for the next merging iteration
        self.bin_size *= 2;
    }

    fn get_last_bins(&mut self) -> Option<(&[T], &[T], &mut [T])> {
        let remainder = self.get_length() % (2 * self.get_bin_size());
        if remainder != 0 {
            let end = self.get_length();
            let mid = end - remainder;
            let start = mid - 2 * self.get_bin_size();
            // Updating the values vector with the sorted bin
            self.values[start..mid].clone_from_slice(&self.buffer[start..mid]);
            let (bin1, bin2, buffer) = self.into_iter().get_bins(start, mid, end);
            Some((bin1, bin2, buffer))
        } else {
            None
        }
    }

    fn get_bin_size(&self) -> usize {
        self.bin_size
    }

    fn get_length(&self) -> usize {
        self.length
    }

    fn get_values(self) -> Vec<T> {
        self.values
    }
}

impl<'a, T: SortTraits> SortVecPairIterMut<'a, T> {
    fn get_bins(
        &mut self,
        start: usize,
        mid: usize,
        end: usize,
    ) -> (&'a [T], &'a [T], &'a mut [T]) {
        // Raw pointer
        let vec_pair_ptr: *mut SortVecPair<T> = self.vec_pair;
        // Unsafe operations here to sidestep the borrow checker.
        // It is safe as long as we use it in only one iterator.
        let bin1 = unsafe { &(*vec_pair_ptr).values[start..mid] };
        let bin2 = unsafe { &(*vec_pair_ptr).values[mid..end] };
        let buffer = unsafe { &mut (*vec_pair_ptr).buffer[start..end] };
        (bin1, bin2, buffer)
    }
}

pub fn merge_sort<T: SortTraits>(input: &[T]) -> Vec<T> {
    let mut sort_vec_pair = SortVecPair::new(input);
    while 2 * sort_vec_pair.get_bin_size() < input.len() {
        for (bin1, bin2, buf) in &mut sort_vec_pair {
            merge_bins(bin1, bin2, buf);
        }
        // Merge remaining values.
        // Put merged bins from the buffer into the values
        // and increase the bins size.
        // Separate from the main operation
        // to ease threading.
        sort_vec_pair.finish_merge();
    }
    sort_vec_pair.get_values()
}

use std::cmp::Ordering;
fn merge_bins<T: SortTraits>(bin1: &[T], bin2: &[T], buf: &mut [T]) {
    let mut id1 = 0;
    let mut id2 = 0;
    for min_val in buf {
        if id1 >= bin1.len() {
            *min_val = bin2[id2].clone();
            id2 += 1;
        } else if id2 >= bin2.len() {
            *min_val = bin1[id1].clone();
            id1 += 1;
        } else {
            let val1 = bin1[id1].clone();
            let val2 = bin2[id2].clone();
            match val1.cmp(&val2) {
                Ordering::Less | Ordering::Equal => {
                    *min_val = val1;
                    id1 += 1;
                }
                Ordering::Greater => {
                    *min_val = val2;
                    id2 += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_bins_test() {
        let vec1 = [4, 5, 6];
        let vec2 = [1, 2, 3];
        let mut vec3 = vec![0; 4];
        let bin1 = &vec1[..2];
        let bin2 = &vec2[1..];
        let buf = &mut vec3[..];
        merge_bins(bin1, bin2, buf);
        assert_eq!(vec3, vec![2, 3, 4, 5]);
    }

    #[test]
    fn sort_small_vec() {
        let test_vec = vec![15, 53, 1, 24, 3];
        assert_eq!(merge_sort(&test_vec), vec![1, 3, 15, 24, 53]);
    }
}
