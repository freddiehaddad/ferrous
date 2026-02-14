use crate::memory::{copy_from_user, copy_to_user};
use crate::net::driver::DRIVER;
use crate::net::ipv4::Ipv4Header;
use crate::net::socket::{process_rx, SOCKETS};
use crate::net::udp::UdpHeader;
use crate::net::SockAddrIn;
use crate::syscall::SyscallReturn;
use crate::thread::tcb::FileDescriptor;
use crate::thread::ThreadManager;
use alloc::vec;
use ferrous_vm::{TrapError, VirtAddr};
use zerocopy::{AsBytes, FromBytes};

pub fn sys_socket(thread_manager: &mut ThreadManager) -> Result<SyscallReturn, TrapError> {
    let mut table = SOCKETS.lock();
    let socket_id = table.create_socket();

    // Add to current thread's FD table
    let current = thread_manager
        .current_thread
        .ok_or(TrapError::HandlerPanic("No thread".into()))?;
    let tcb = thread_manager.threads.get_mut(&current).unwrap();

    // Find free FD
    for (i, fd) in tcb.file_descriptors.iter_mut().enumerate() {
        if fd.is_none() {
            *fd = Some(FileDescriptor::Socket { socket_id });
            return Ok(SyscallReturn::Value(i as i64));
        }
    }

    // Expand
    tcb.file_descriptors
        .push(Some(FileDescriptor::Socket { socket_id }));
    Ok(SyscallReturn::Value(
        (tcb.file_descriptors.len() - 1) as i64,
    ))
}

pub fn sys_bind(
    thread_manager: &mut ThreadManager,
    memory: &mut dyn ferrous_vm::Memory,
    fd: usize,
    ptr: VirtAddr,
    len: usize,
) -> Result<SyscallReturn, TrapError> {
    let current = thread_manager
        .current_thread
        .ok_or(TrapError::HandlerPanic("No thread".into()))?;
    let tcb = thread_manager.threads.get_mut(&current).unwrap();
    let satp = tcb.context.satp;

    if fd >= tcb.file_descriptors.len() {
        return Ok(SyscallReturn::Value(-1));
    }

    // Read SockAddrIn from user
    if len < core::mem::size_of::<SockAddrIn>() {
        return Ok(SyscallReturn::Value(-1));
    }
    let mut addr_bytes = vec![0u8; core::mem::size_of::<SockAddrIn>()];
    copy_from_user(memory, satp, ptr, &mut addr_bytes)?;

    let sockaddr = SockAddrIn::read_from(&addr_bytes[..])
        .ok_or(TrapError::HandlerPanic("Invalid sockaddr".into()))?;
    let port = u16::from_be(sockaddr.port);

    if let Some(FileDescriptor::Socket { socket_id }) = tcb.file_descriptors[fd] {
        let mut table = SOCKETS.lock();
        if table.bind(socket_id, port) {
            Ok(SyscallReturn::Value(0))
        } else {
            Ok(SyscallReturn::Value(-1))
        }
    } else {
        Ok(SyscallReturn::Value(-1))
    }
}

pub fn sys_sendto(
    thread_manager: &mut ThreadManager,
    memory: &mut dyn ferrous_vm::Memory,
    fd: usize,
    buf_ptr: VirtAddr,
    len: usize,
    dest_ptr: VirtAddr,
    dest_len: usize,
) -> Result<SyscallReturn, TrapError> {
    let current = thread_manager
        .current_thread
        .ok_or(TrapError::HandlerPanic("No thread".into()))?;
    let tcb = thread_manager.threads.get_mut(&current).unwrap();
    let satp = tcb.context.satp;

    if fd >= tcb.file_descriptors.len() {
        return Ok(SyscallReturn::Value(-1));
    }

    let socket_id = match tcb.file_descriptors[fd] {
        Some(FileDescriptor::Socket { socket_id }) => socket_id,
        _ => return Ok(SyscallReturn::Value(-1)),
    };

    // Read payload
    let mut data = vec![0u8; len];
    copy_from_user(memory, satp, buf_ptr, &mut data)?;

    // Read Dest SockAddr
    if dest_len < core::mem::size_of::<SockAddrIn>() {
        return Ok(SyscallReturn::Value(-1));
    }
    let mut addr_bytes = vec![0u8; core::mem::size_of::<SockAddrIn>()];
    copy_from_user(memory, satp, dest_ptr, &mut addr_bytes)?;
    let sockaddr = SockAddrIn::read_from(&addr_bytes[..])
        .ok_or(TrapError::HandlerPanic("Invalid sockaddr".into()))?;

    let dest_port = u16::from_be(sockaddr.port);
    let dest_ip_bytes = sockaddr.addr.to_be_bytes();

    let mut table = SOCKETS.lock();
    let src_port = if let Some(sock) = table.get_socket(socket_id) {
        sock.local_port
    } else {
        0
    };
    drop(table); // Unlock

    // Construct Packet
    // 14 Eth + 20 IP + 8 UDP + Data
    let total_len = 14 + 20 + 8 + len;
    let mut packet = vec![0u8; total_len];

    // Ethernet
    // Dest MAC: 52:54:00:12:34:56 (Host)
    packet[0..6].copy_from_slice(&[0x52, 0x54, 0x00, 0x12, 0x34, 0x56]);
    // Src MAC: 52:54:00:12:34:56 (VM - same? or slightly diff? device ignores src usually)
    packet[6..12].copy_from_slice(&[0x52, 0x54, 0x00, 0x12, 0x34, 0x57]);
    // Type: IPv4
    packet[12] = 0x08;
    packet[13] = 0x00;

    // IP
    let mut ip_header = Ipv4Header::new(
        [10, 0, 2, 15], // QEMU User Net standard guest IP
        dest_ip_bytes,
        17, // UDP
        (20 + 8 + len) as u16,
    );
    ip_header.calculate_checksum();
    packet[14..34].copy_from_slice(ip_header.as_bytes());

    // UDP
    let udp_header = UdpHeader::new(src_port, dest_port, (8 + len) as u16);
    packet[34..42].copy_from_slice(udp_header.as_bytes());

    // Payload
    packet[42..].copy_from_slice(&data);

    // Send
    DRIVER.lock().send_packet(&packet);

    Ok(SyscallReturn::Value(len as i64))
}

pub fn sys_recvfrom(
    thread_manager: &mut ThreadManager,
    memory: &mut dyn ferrous_vm::Memory,
    fd: usize,
    buf_ptr: VirtAddr,
    len: usize,
    src_ptr: VirtAddr,
    src_len_ptr: VirtAddr,
) -> Result<SyscallReturn, TrapError> {
    // Poll driver first
    process_rx();

    let current = thread_manager
        .current_thread
        .ok_or(TrapError::HandlerPanic("No thread".into()))?;
    let tcb = thread_manager.threads.get_mut(&current).unwrap();
    let satp = tcb.context.satp;

    if fd >= tcb.file_descriptors.len() {
        return Ok(SyscallReturn::Value(-1));
    }

    let socket_id = match tcb.file_descriptors[fd] {
        Some(FileDescriptor::Socket { socket_id }) => socket_id,
        _ => return Ok(SyscallReturn::Value(-1)),
    };

    let mut table = SOCKETS.lock();
    if let Some(sock) = table.get_socket(socket_id) {
        if let Some(packet) = sock.rx_queue.pop_front() {
            let copy_len = core::cmp::min(packet.payload.len(), len);
            copy_to_user(memory, satp, &packet.payload[..copy_len], buf_ptr)?;

            // Write source address if requested
            if src_ptr.val() != 0 {
                let src_addr = SockAddrIn {
                    family: 2,
                    port: packet.src_port.to_be(),
                    addr: u32::from_be_bytes(packet.src_ip).to_be(), // src_ip is bytes. u32::from_be_bytes creates integer. .to_be() swaps it for Network Order?
                    // src_ip is [u8; 4] (e.g. 10.0.2.2 -> [10, 0, 2, 2]).
                    // u32::from_be_bytes([10, 0, 2, 2]) -> 0x0A000202 (on any arch).
                    // SockAddrIn.addr is u32. We want to store 0x0A000202.
                    // But if we write struct bytes to memory (LE), it writes 02 02 00 0A.
                    // Wait, `AsBytes` writes bytes in native order.
                    // If struct field is u32 = 0x0A000202. On LE, bytes are 02 02 00 0A.
                    // Network expects 0A 00 02 02.
                    // So we need `addr` to be `0x0202000A` (swapped) so that when written as LE bytes, it comes out as `0A 00 02 02`?
                    // YES.
                    // u32::from_be_bytes([10, 0, 2, 2]) = 0x0A000202.
                    // .to_be() on LE -> 0x0202000A.
                    // Correct.
                    zero: [0; 8],
                };

                let addr_bytes = src_addr.as_bytes();
                copy_to_user(memory, satp, addr_bytes, src_ptr)?;

                // Update length
                if src_len_ptr.val() != 0 {
                    let len_val = (addr_bytes.len() as u32).to_le_bytes(); // User expects LE size_t
                    copy_to_user(memory, satp, &len_val, src_len_ptr)?;
                }
            }

            return Ok(SyscallReturn::Value(copy_len as i64));
        }
    }

    Ok(SyscallReturn::Value(-1)) // Would block
}
