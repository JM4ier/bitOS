/// Copies the first `size` elements from `input` to `output`
#[inline]
pub fn copy<T: Copy>(input: &[T], output: &mut [T], size: usize) {
    copy_offset(input, output, size, 0, 0);
}

/// Copies `size` elements beginning with the `ioffset` (input offset) element from `input` to the elements in `output`
/// beginning with `ooffset` (output offset)
#[inline]
pub fn copy_offset<T: Copy>(input: &[T], output: &mut [T], size: usize, ioffset: usize, ooffset: usize) {
    for i in 0..size {
        output[i + ooffset] = input[ioffset + i];
    }
}

