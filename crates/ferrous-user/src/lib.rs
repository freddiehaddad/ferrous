#![no_std]

use core::fmt;

pub mod syscall {
    use core::arch::asm;

    pub fn console_write(fd: u32, buf: &[u8]) {
        let ptr = buf.as_ptr();
        let len = buf.len();
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a0") fd,
                in("a1") ptr,
                in("a2") len,
                in("a7") 64,
            );
        }
        #[cfg(not(target_arch = "riscv32"))]
        {
            // Mock for host testing if needed, or panic
            let _ = (fd, ptr, len);
        }
    }

    pub fn exit(code: i32) -> ! {
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a0") code,
                in("a7") 93,
            );
        }
        #[cfg(not(target_arch = "riscv32"))]
        {
            let _ = code;
            loop {}
        }
        #[allow(unreachable_code)]
        loop {}
    }
}

pub struct Console;

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        syscall::console_write(1, s.as_bytes());
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    use fmt::Write;
    Console.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

pub fn exit(code: i32) -> ! {
    syscall::exit(code)
}
