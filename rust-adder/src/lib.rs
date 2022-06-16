#[no_mangle]
pub extern "C" fn rust_add_numbers(a: i32, b: i32) -> i32 {
    return a + b;
}
