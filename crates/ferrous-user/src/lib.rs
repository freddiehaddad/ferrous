#![no_std]

use core::fmt;

pub mod sync;

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
        unsafe {
            #[cfg(target_arch = "riscv32")]
            asm!(
                "ecall",
                in("a0") code,
                in("a7") 93,
                options(noreturn)
            );
            #[cfg(not(target_arch = "riscv32"))]
            loop {}
        }
    }

    pub fn thread_yield() {
        unsafe {
            #[cfg(target_arch = "riscv32")]
            asm!(
                "ecall",
                in("a7") 101,
            );
        }
    }

    pub fn thread_create(entry: usize, stack_top: usize) -> u32 {
        let ret: u32;
        unsafe {
            #[cfg(target_arch = "riscv32")]
            asm!(
                "ecall",
                in("a0") entry,
                in("a1") stack_top,
                in("a7") 102,
                lateout("a0") ret,
            );
            #[cfg(not(target_arch = "riscv32"))]
            {
                let _ = (entry, stack_top);
                ret = 0;
            }
        }
        ret
    }

    pub fn mutex_create() -> u32 {
        let ret: u32;
        unsafe {
            #[cfg(target_arch = "riscv32")]
            asm!(
                "ecall",
                in("a7") 110,
                lateout("a0") ret,
            );
            #[cfg(not(target_arch = "riscv32"))]
            {
                ret = 0;
            }
        }
        ret
    }

    pub fn mutex_acquire(id: u32) {
        unsafe {
            #[cfg(target_arch = "riscv32")]
            asm!(
                "ecall",
                in("a0") id,
                in("a7") 111,
            );
        }
    }

    pub fn mutex_release(id: u32) {
        unsafe {
            #[cfg(target_arch = "riscv32")]
            asm!(
                "ecall",
                in("a0") id,
                in("a7") 112,
            );
        }
    }

    pub fn sbrk(increment: i32) -> u32 {
        let ret: u32;
        unsafe {
            #[cfg(target_arch = "riscv32")]
            asm!(
                "ecall",
                in("a0") increment,
                in("a7") 214,
                lateout("a0") ret,
            );
            #[cfg(not(target_arch = "riscv32"))]
            {
                ret = 0;
            }
        }
        ret
    }

    pub fn block_read(sector: u32, buf: &mut [u8]) -> Result<(), u32> {
        let ptr = buf.as_mut_ptr();
        let ret: u32;
        unsafe {
            #[cfg(target_arch = "riscv32")]
            asm!(
                "ecall",
                in("a0") sector,
                in("a1") ptr,
                in("a7") 200,
                lateout("a0") ret,
            );
            #[cfg(not(target_arch = "riscv32"))]
            {
                ret = 0;
            }
        }
        if ret == 0 {
            Ok(())
        } else {
            Err(ret)
        }
    }
}

pub struct Console;

// We need a way to initialize this lazily or statically.
// Since we don't have atomic/lazy_static easily in no_std without support,
// we'll rely on a dedicated syscall to lock the console, OR
// we expose a Mutex to the user.
// But println! is a macro.
// For now, let's just make console_write atomic in the kernel?
// No, console_write IS atomic (one buffer).
// The problem is `write_fmt` calls `write_str` multiple times.
// We need to lock AROUND write_fmt.

// Hack: Global boolean flag? No, race condition.
// Real solution: Global Mutex initialized at start.
// But we can't run code at start easily (pre-main).
// We can have `ferrous_user_init()` called by `_start`.

static mut CONSOLE_MUTEX_ID: u32 = 0;

pub fn init() {
    unsafe {
        CONSOLE_MUTEX_ID = syscall::mutex_create();
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        syscall::console_write(1, s.as_bytes());
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    use fmt::Write;
    unsafe {
        if CONSOLE_MUTEX_ID != 0 {
            syscall::mutex_acquire(CONSOLE_MUTEX_ID);
        }
    }
    Console.write_fmt(args).unwrap();
    unsafe {
        if CONSOLE_MUTEX_ID != 0 {
            syscall::mutex_release(CONSOLE_MUTEX_ID);
        }
    }
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

pub fn yield_now() {
    syscall::thread_yield();
}

extern "C" {
    static _end: u8;
}

static mut HEAP_PTR: usize = 0;
const STACK_SIZE: usize = 4096;

fn alloc_stack() -> usize {
    unsafe {
        if HEAP_PTR == 0 {
            HEAP_PTR = &_end as *const u8 as usize;
            // Align to 16 bytes
            HEAP_PTR = (HEAP_PTR + 15) & !15;
        }

        let stack_bottom = HEAP_PTR;
        HEAP_PTR += STACK_SIZE;
        // TODO: Check for OOM against top of RAM

        // Stack grows down, so return top
        stack_bottom + STACK_SIZE
    }
}

pub fn spawn(entry: extern "C" fn()) -> u32 {
    let stack_top = alloc_stack();
    syscall::thread_create(entry as usize, stack_top)
}
