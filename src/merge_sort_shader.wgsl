@group(0) @binding(0) var<storage, read> input: array<i32>;
@group(0) @binding(1) var<storage, read_write> output: array<i32>;
@group(0) @binding(2) var<storage, read> bin_size: u32;

@compute
@workgroup_size(64, 1, 1)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    let vec_len = arrayLength(&input);
    let start = gid.x * 2 * bin_size;

    if (start > vec_len) {
        return;
    }

    let mid = start + bin_size;
    var end = mid + bin_size;

    if mid > vec_len {
        return;
    } else if end > vec_len {
        end = vec_len;
    }

    var id1 = start;
    var id2 = mid;
    var idout = start;

    while idout < end {
        if id1 >= mid {
            output[idout] = input[id2];
            id2 += 1;
        } else if id2 >= end {
            output[idout] = input[id1];
            id1 += 1;
        } else {
            let val1 = input[id1];
            let val2 = input[id2];
            if val1 <= val2 {
                output[idout] = val1;
                id1 += 1;
            } else {
                output[idout] = val2;
                id2 += 1;
            }
        }
        idout += 1;
    }
}
