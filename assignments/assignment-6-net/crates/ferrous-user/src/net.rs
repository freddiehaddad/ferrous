use crate::syscall;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SockAddrIn {
    pub family: u16,
    pub port: u16,
    pub addr: u32,
    pub zero: [u8; 8],
}

impl SockAddrIn {
    pub fn new(addr: u32, port: u16) -> Self {
        // Simple default constructor, careful with endianness
        Self {
            family: 2, // AF_INET
            port, // User handles endianness or we do helper? Let's assume user gives host order?
            // Standard socket API expects network byte order in the struct.
            // Let's provide a helper that takes host order and converts.
            addr,
            zero: [0; 8],
        }
    }
}

pub fn socket(domain: u32, type_: u32, protocol: u32) -> Result<u32, i32> {
    let ret = syscall::socket(domain, type_, protocol);
    // In our kernel, socket returns the FD (u32), or high error?
    // Let's assume it returns u32::MAX on error or similar.
    // Wait, sys_socket returns Result<u32, u32> in kernel.
    // The ASM wrapper returns u32.
    if ret > 2000000000 {
        // Heuristic for negative/error code if treated as signed
        Err(-1)
    } else {
        Ok(ret)
    }
}

pub fn bind(sockfd: u32, addr: &SockAddrIn) -> Result<(), i32> {
    let ret = syscall::bind(
        sockfd,
        addr as *const _ as *const u8,
        core::mem::size_of::<SockAddrIn>() as u32,
    );
    if ret < 0 {
        Err(ret)
    } else {
        Ok(())
    }
}

pub fn sendto(sockfd: u32, buf: &[u8], flags: u32, dest_addr: &SockAddrIn) -> Result<usize, i32> {
    let ret = syscall::sendto(
        sockfd,
        buf.as_ptr(),
        buf.len(),
        flags,
        dest_addr as *const _ as *const u8,
        core::mem::size_of::<SockAddrIn>() as u32,
    );
    if ret < 0 {
        Err(ret)
    } else {
        Ok(ret as usize)
    }
}

pub fn recvfrom(sockfd: u32, buf: &mut [u8], flags: u32) -> Result<(usize, SockAddrIn), i32> {
    let mut src_addr = SockAddrIn {
        family: 0,
        port: 0,
        addr: 0,
        zero: [0; 8],
    };
    let mut addrlen = core::mem::size_of::<SockAddrIn>() as u32;

    let ret = syscall::recvfrom(
        sockfd,
        buf.as_mut_ptr(),
        buf.len(),
        flags,
        &mut src_addr as *mut _ as *mut u8,
        &mut addrlen,
    );

    if ret < 0 {
        Err(ret)
    } else {
        Ok((ret as usize, src_addr))
    }
}

pub fn htons(u: u16) -> u16 {
    u.to_be()
}

pub fn htonl(u: u32) -> u32 {
    u.to_be()
}

pub fn ntohs(u: u16) -> u16 {
    u16::from_be(u)
}

pub fn ntohl(u: u32) -> u32 {
    u32::from_be(u)
}
