#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(snic_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use snic_os::println;

#[no_mangle] // この関数の名前を変えない
pub extern "C" fn _start() -> ! {
    test_main();

    loop {}
}

fn test_runner(tests: &[&dyn Fn()]) {
    unimplemented!();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    snic_os::test_panic_handler(info)
}

#[test_case]
fn test_println() {
    println!("test_println output");
}