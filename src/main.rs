use merge_sort::merge_sort;

fn main() {
    //let test_vec = vec![15, 53, 1, 24, 3];
    let test_vec = vec![15, 53, 1, 24, 3, 1765, 22, 2, 8, 7, 4];
    let sorted_vec = merge_sort(&test_vec);
    println!("Sorted vec: {sorted_vec:?}");
}
