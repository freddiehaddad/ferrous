#![allow(dead_code)]

use crate::syscall;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SockAddrIn {
    pub family: u16,
    pub port: u16, // Big Endian
    pub addr: u32, // Big Endian
    pub zero: [u8; 8],
}

impl SockAddrIn {
    pub fn new(port: u16, addr: u32) -> Self {
        Self {
            family: 2, // AF_INET
            port: htons(port),
            addr: htonl(addr),
            zero: [0; 8],
        }
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

/// Create a UDP socket. Returns file descriptor or error code.
pub fn socket() -> Result<i32, i32> {
    syscall::socket()
}

/// Bind socket to address.
pub fn bind(fd: i32, addr: &SockAddrIn) -> Result<(), i32> {
    let ret = syscall::bind(
        fd as u32,
        addr as *const _ as *const u8,
        core::mem::size_of::<SockAddrIn>() as u32,
    );
    if ret == 0 {
        Ok(())
    } else {
        Err(ret)
    }
}

/// Send data to address.
pub fn sendto(fd: i32, buf: &[u8], addr: &SockAddrIn) -> Result<usize, i32> {
    let ret = syscall::sendto(
        fd as u32,
        buf.as_ptr(),
        buf.len() as u32,
        addr as *const _ as *const u8,
        core::mem::size_of::<SockAddrIn>() as u32,
    );
    if ret >= 0 {
        Ok(ret as usize)
    } else {
        Err(ret)
    }
}

/// Receive data from socket. Returns (bytes_read, src_addr).
pub fn recvfrom(fd: i32, buf: &mut [u8]) -> Result<(usize, SockAddrIn), i32> {
    let mut src_addr = SockAddrIn {
        family: 0,
        port: 0,
        addr: 0,
        zero: [0; 8],
    };
    let mut addr_len: u32 = core::mem::size_of::<SockAddrIn>() as u32;

    let ret = syscall::recvfrom(
        fd as u32,
        buf.as_mut_ptr(),
        buf.len() as u32,
        &mut src_addr as *mut _ as *mut u8,
        &mut addr_len as *mut u32,
    );

    if ret >= 0 {
        Ok((ret as usize, src_addr))
    } else {
        Err(ret)
    }
}
