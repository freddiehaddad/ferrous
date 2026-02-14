#![no_std]

use core::fmt;

pub mod sync;

pub mod syscall {
    #[cfg(target_arch = "riscv32")]
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

    pub fn console_read(_fd: u32, buf: &mut [u8]) -> usize {
        let _ptr = buf.as_mut_ptr();
        let _len = buf.len();
        let ret: usize;
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a0") _fd,
                in("a1") _ptr,
                in("a2") _len,
                in("a7") 65,
                lateout("a0") ret,
            );
        }
        #[cfg(not(target_arch = "riscv32"))]
        {
            ret = 0;
        }
        ret
    }

    pub fn exit(_code: i32) -> ! {
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a0") _code,
                in("a7") 93,
                options(noreturn)
            );
        }
        #[cfg(not(target_arch = "riscv32"))]
        panic!("exit called on host");
    }

    pub fn thread_yield() {
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a7") 101,
            );
        }
    }

    pub fn thread_create(entry: usize, stack_top: usize) -> u32 {
        let ret: u32;
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a0") entry,
                in("a1") stack_top,
                in("a7") 102,
                lateout("a0") ret,
            );
        }
        #[cfg(not(target_arch = "riscv32"))]
        {
            let _ = (entry, stack_top);
            ret = 0;
        }
        ret
    }

    pub fn mutex_create() -> u32 {
        let ret: u32;
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a7") 110,
                lateout("a0") ret,
            );
        }
        #[cfg(not(target_arch = "riscv32"))]
        {
            ret = 0;
        }
        ret
    }

    pub fn mutex_acquire(_id: u32) {
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a0") _id,
                in("a7") 111,
            );
        }
    }

    pub fn mutex_release(_id: u32) {
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a0") _id,
                in("a7") 112,
            );
        }
    }

    pub fn sbrk(_increment: i32) -> u32 {
        let ret: u32;
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a0") _increment,
                in("a7") 214,
                lateout("a0") ret,
            );
        }
        #[cfg(not(target_arch = "riscv32"))]
        {
            ret = 0;
        }
        ret
    }

    pub fn block_read(_sector: u32, buf: &mut [u8]) -> Result<(), u32> {
        let _ptr = buf.as_mut_ptr();
        let ret: u32;
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a0") _sector,
                in("a1") _ptr,
                in("a7") 200,
                lateout("a0") ret,
            );
        }
        #[cfg(not(target_arch = "riscv32"))]
        {
            ret = 0;
        }
        if ret == 0 {
            Ok(())
        } else {
            Err(ret)
        }
    }

    pub fn file_open(path: &str) -> Result<u32, u32> {
        let _ptr = path.as_ptr();
        let _len = path.len();
        let ret: u32;
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a0") _ptr,
                in("a1") _len,
                in("a7") 56,
                lateout("a0") ret,
            );
        }
        #[cfg(not(target_arch = "riscv32"))]
        {
            ret = 0xFFFFFFFF;
        }
        if ret != u32::MAX {
            Ok(ret)
        } else {
            Err(ret)
        }
    }

    pub fn file_read(_fd: u32, buf: &mut [u8]) -> Result<usize, u32> {
        let _ptr = buf.as_mut_ptr();
        let _len = buf.len();
        let ret: usize;
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a0") _fd,
                in("a1") _ptr,
                in("a2") _len,
                in("a7") 63,
                lateout("a0") ret,
            );
        }
        #[cfg(not(target_arch = "riscv32"))]
        {
            ret = 0;
        }
        if ret != u32::MAX as usize {
            Ok(ret)
        } else {
            Err(0)
        }
    }

    pub fn file_write(_fd: u32, buf: &[u8]) -> Result<usize, u32> {
        let _ptr = buf.as_ptr();
        let _len = buf.len();
        let ret: usize;
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a0") _fd,
                in("a1") _ptr,
                in("a2") _len,
                in("a7") 64, // Using write syscall (same as console_write, but kernel dispatches based on FD)
                lateout("a0") ret,
            );
        }
        #[cfg(not(target_arch = "riscv32"))]
        {
            ret = 0;
        }
        if ret != u32::MAX as usize {
            Ok(ret)
        } else {
            Err(0)
        }
    }

    pub fn file_close(_fd: u32) {
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a0") _fd,
                in("a7") 57,
            );
        }
    }

    pub fn exec(path: &str, args: &[&str]) -> Result<u32, u32> {
        let _ptr = path.as_ptr();
        let _len = path.len();
        let _args_ptr = args.as_ptr();
        let _args_len = args.len();
        let ret: u32;
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a0") _ptr,
                in("a1") _len,
                in("a2") _args_ptr,
                in("a3") _args_len,
                in("a7") 59,
                lateout("a0") ret,
            );
        }
        #[cfg(not(target_arch = "riscv32"))]
        {
            let _ = (_args_ptr, _args_len);
            ret = 0xFFFFFFFF;
        }
        if ret != u32::MAX {
            Ok(ret)
        } else {
            Err(ret)
        }
    }

    pub fn pipe(fds: &mut [u32; 2]) -> Result<(), u32> {
        let ret: u32;
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a0") fds.as_mut_ptr(),
                in("a7") 22,
                lateout("a0") ret,
            );
        }
        #[cfg(not(target_arch = "riscv32"))]
        {
            let _ = fds;
            ret = 0xFFFFFFFF;
        }
        if ret == 0 {
            Ok(())
        } else {
            Err(ret)
        }
    }

    pub fn waitpid(_pid: u32) -> i32 {
        let ret: i32;
        #[cfg(target_arch = "riscv32")]
        unsafe {
            asm!(
                "ecall",
                in("a0") _pid,
                in("a7") 260,
                lateout("a0") ret,
            );
        }
        #[cfg(not(target_arch = "riscv32"))]
        {
            ret = 0;
        }
        ret
    }
}

pub struct Console;

// BufferedConsole to avoid interleaved syscalls
struct BufferedConsole {
    buf: [u8; 256],
    cursor: usize,
}

impl BufferedConsole {
    fn new() -> Self {
        Self {
            buf: [0; 256],
            cursor: 0,
        }
    }

    fn flush(&mut self) {
        if self.cursor > 0 {
            syscall::console_write(1, &self.buf[0..self.cursor]);
            self.cursor = 0;
        }
    }
}

impl fmt::Write for BufferedConsole {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut bytes = s.as_bytes();

        while !bytes.is_empty() {
            let space = self.buf.len() - self.cursor;
            if space == 0 {
                self.flush();
                continue;
            }

            let count = bytes.len().min(space);
            self.buf[self.cursor..self.cursor + count].copy_from_slice(&bytes[0..count]);
            self.cursor += count;
            bytes = &bytes[count..];
        }
        Ok(())
    }
}

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

    // Use stack buffer to prevent interleaving
    let mut buffer = BufferedConsole::new();
    let _ = buffer.write_fmt(args);
    buffer.flush();
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
    // static _end: u8; // Not needed with sbrk
}

// static mut HEAP_PTR: usize = 0; // Not needed with sbrk
const STACK_SIZE: usize = 4096;

fn alloc_stack() -> usize {
    // Allocate STACK_SIZE bytes using sbrk
    // This ensures the kernel maps the pages
    let stack_bottom = syscall::sbrk(STACK_SIZE as i32) as usize;

    // Debug print
    // println!("Allocated stack at {:#x}, top {:#x}", stack_bottom, stack_bottom + STACK_SIZE);
    // Can't use println here easily if it uses stack? No, println uses global stdout, but formats on stack.
    // If we are main thread, we have stack.

    // Stack grows down, so return top
    stack_bottom + STACK_SIZE
}

pub fn spawn(entry: extern "C" fn()) -> u32 {
    let stack_top = alloc_stack();
    syscall::thread_create(entry as usize, stack_top)
}
