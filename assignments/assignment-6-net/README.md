# Assignment 6: Networking

In this assignment, you will implement the network stack system calls for Ferrous OS.
We have provided a simulated network device (based on a simplified virtio-net) and a partial network stack (IPv4/UDP).

## Objective

Your task is to implement the following system calls in `ferrous-kernel/src/net/syscalls.rs`:

1.  `sys_socket(domain, type, protocol)`: Create a new UDP socket.
2.  `sys_bind(sockfd, addr, addrlen)`: Bind a socket to a local port.
3.  `sys_sendto(sockfd, buf, len, flags, dest_addr, addrlen)`: Send a UDP packet.
4.  `sys_recvfrom(sockfd, buf, len, flags, src_addr, addrlen)`: Receive a UDP packet.

## Architecture

*   **Device Driver**: `ferrous-kernel/src/net/driver.rs` handles the low-level MMIO with the simulated NIC.
*   **Protocols**: `ferrous-kernel/src/net/ipv4.rs` and `udp.rs` handle packet formatting.
*   **Sockets**: `ferrous-kernel/src/net/socket.rs` manages the socket table and RX queues.

## The Simulated Network

The Ferrous VM now includes a `SimpleNetDevice` at address `0x3000_0000`.
It acts as a tunnel:
*   Anything sent by the VM is forwarded to a "Host Server" on `127.0.0.1:5555`.
*   Anything sent by the "Host Server" to the VM is received by the driver and placed in the RX buffer.

## Verification

We have provided a user-space program `net_test` to verify your implementation.
It sends a "Hello" packet to `10.0.2.2` (the Host) and expects a response.

To run the verification:

```bash
cargo xtask run-net
```

This command will:
1.  Compile the kernel and user programs.
2.  Start a temporary "Host Server" listening on port 5555.
3.  Launch the VM running `net_test`.
4.  Verify the handshake.

## Reference

*   **IPv4 Header**: 20 bytes.
*   **UDP Header**: 8 bytes.
*   **Host IP (Simulated)**: 10.0.2.2
*   **Guest IP (Simulated)**: 10.0.2.15

Good luck!
