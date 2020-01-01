pub fn copy<T: Copy>(input: &[T], output: &mut [T], size: usize) {
    copy_offset(input, output, size, 0, 0);
}

pub fn copy_offset<T: Copy>(input: &[T], output: &mut [T], size: usize, ioffset: usize, ooffset: usize) {
    for i in 0..size {
        output[i + ooffset] = input[ioffset + i];
    }
}
