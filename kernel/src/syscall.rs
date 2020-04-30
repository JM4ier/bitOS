use crate::println;


#[no_mangle]
pub unsafe extern "C" fn __syscall(rdi: u64, rsi: u64, rdx: u64, rcx: u64, r8: u64, r9: u64) -> i64 {
    println!("received a syscall!");
    0
}

