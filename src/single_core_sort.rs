// Trait aliasing for readibility
// https://stackoverflow.com/questions/26070559/is-there-any-way-to-create-a-type-alias-for-multiple-trai  ts
pub trait SortTraits: Clone + PartialOrd {}
impl<T: Clone + PartialOrd> SortTraits for T {}
struct SortVecPair<T: SortTraits> {
    bin_size: usize,
    length: usize,
    values: Vec<T>,
    buffer: Vec<T>,
}
struct BinsPositions {
    start: usize,
    mid: usize,
    end: usize,
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
        // Reinject the buffer in the values
        self.values = self.buffer.clone();
        // Double the bin size to prepare for the next merging iteration
        self.bin_size *= 2;
    }

    fn get_bin_size(&self) -> usize {
        self.bin_size
    }

    fn get_values(self) -> Vec<T> {
        self.values
    }
    fn get_bins_positions(&self, end_prev: usize) -> Option<BinsPositions> {
        if end_prev + 2 * self.bin_size < self.length {
            Some(BinsPositions {
                start: end_prev,
                mid: end_prev + self.bin_size,
                end: end_prev + 2 * self.bin_size,
            })
        } else if end_prev + self.bin_size < self.length {
            Some(BinsPositions {
                start: end_prev,
                mid: end_prev + self.bin_size,
                end: self.length,
            })
        } else {
            None
        }
    }
}
pub fn merge_sort<T: SortTraits>(input: &[T]) -> Vec<T> {
    let mut sort_vec_pair = SortVecPair::new(input);
    while sort_vec_pair.get_bin_size() < input.len() {
        let mut end_prev = 0;
        while let Some(BinsPositions { start, mid, end }) =
            sort_vec_pair.get_bins_positions(end_prev)
        {
            let bin1 = &sort_vec_pair.values[start..mid];
            let bin2 = &sort_vec_pair.values[mid..end];
            let buf = &mut sort_vec_pair.buffer[start..end];
            merge_bins(bin1, bin2, buf);
            end_prev = end;
        }
        // Put merged bins from the buffer into the values
        // and increase the bins size.
        // Separate from the main operation
        // to ease threading.
        sort_vec_pair.finish_merge();
    }
    sort_vec_pair.get_values()
}
fn merge_bins<T: SortTraits>(bin1: &[T], bin2: &[T], buf: &mut [T]) {
    let mut id1 = 0;
    let mut id2 = 0;
    for min_val in buf {
        if id1 >= bin1.len() {
            *min_val = bin2[id2].clone();
            id2 += 1;
        }else if id2 >= bin2.len() {
            *min_val = bin1[id1].clone();
            id1 += 1;
        } else {
            let val1 = bin1[id1].clone();
            let val2 = bin2[id2].clone();
            if val1 <= val2 {
                *min_val = val1;
                id1 += 1;
            } else {
                *min_val = val2;
                id2 += 1;
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

    #[test]
    fn sort_small_vec_float() {
        let test_vec = vec![15.1, 15.3, 53.2, 1.9, 1.5, 24.7, 3.2];
        assert_eq!(
            merge_sort(&test_vec),
            vec![1.5, 1.9, 3.2, 15.1, 15.3, 24.7, 53.2]
        );
    }
}
