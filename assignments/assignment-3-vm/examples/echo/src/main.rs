#![no_std]
#![no_main]

extern crate ferrous_user;
use ferrous_user::{print, println};

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

/// # Safety
///
/// This function is the entry point for the program and is responsible for
/// initializing the environment. It interprets raw raw pointers from the
/// kernel, which must be valid.
#[no_mangle]
pub unsafe extern "C" fn _start(argc: usize, argv: *const &str) -> ! {
    ferrous_user::init();

    // Iterate args
    // Note: In standard C/Unix, argv[0] is the program name.
    // In our shell implementation, we passed "program" separately to exec.
    // But exec just loaded the ELF.
    // The args passed to exec were the *rest* of the command line.
    // So argv[0] here will be the first argument, not the program name!
    // Unless the Shell prepended the program name to args.
    // Let's check the shell code:
    // let args = &parts[1..];
    // exec(program, args)
    // So argc will be 0 if we just type "echo".

    // Standard convention suggests argv[0] should be program name.
    for i in 0..argc {
        let arg = unsafe { *argv.add(i) };
        print!("{} ", arg);
    }
    println!("");

    ferrous_user::exit(0);
}
