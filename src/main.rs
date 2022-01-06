#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(snic_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use snic_os::println;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello SnicOS{}", "!");

    #[cfg(test)]
        test_main();

    loop {}
}

/// この関数はパニック時に呼ばれる。
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    snic_os::test_panic_handler(info)
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}