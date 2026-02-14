use crate::devices::{Device, DeviceInterrupt};
use crate::error::DeviceError;

#[cfg(feature = "std")]
use std::net::UdpSocket;

pub const NET_DEVICE_BASE: u32 = 0x3000_0000;
pub const NET_BUFFER_SIZE: usize = 2048;

// Register Offsets
const REG_STATUS: u32 = 0x00; // Read: 1=Data Ready, 0=Idle
const REG_COMMAND: u32 = 0x04; // Write: 1=Send, 2=Recv
const REG_LENGTH: u32 = 0x08; // RW: Packet Length
const REG_MAC_LOW: u32 = 0x10; // Read: MAC Address Lower 32-bits
const REG_MAC_HIGH: u32 = 0x14; // Read: MAC Address Upper 16-bits

// Buffer Window at Offset 0x100
const BUFFER_OFFSET: u32 = 0x100;

#[cfg(not(feature = "std"))]
pub struct SimpleNetDevice;

#[cfg(feature = "std")]
pub struct SimpleNetDevice {
    socket: UdpSocket,
    rx_buffer: [u8; NET_BUFFER_SIZE],
    tx_buffer: [u8; NET_BUFFER_SIZE],
    rx_packet_len: u32,
    tx_packet_len: u32,
    data_ready: bool,
    mac: [u8; 6],
}

#[cfg(feature = "std")]
impl SimpleNetDevice {
    pub fn new(bind_addr: &str, remote_addr: &str) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(bind_addr)?;
        socket.set_nonblocking(true)?;

        // Connect allows send() without addr, creating a "tunnel"
        if !remote_addr.is_empty() {
            socket.connect(remote_addr)?;
        }

        Ok(Self {
            socket,
            rx_buffer: [0; NET_BUFFER_SIZE],
            tx_buffer: [0; NET_BUFFER_SIZE],
            rx_packet_len: 0,
            tx_packet_len: 0,
            data_ready: false,
            mac: [0x52, 0x54, 0x00, 0x12, 0x34, 0x56], // Standard QEMU MAC
        })
    }

    fn check_rx(&mut self) {
        if !self.data_ready {
            match self.socket.recv(&mut self.rx_buffer) {
                Ok(len) => {
                    println!("[VM Net] Received {} bytes from host socket", len);
                    self.rx_packet_len = len as u32;
                    self.data_ready = true;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No data
                }
                Err(_) => {
                    // Error
                }
            }
        }
    }
}

#[cfg(feature = "std")]
impl Device for SimpleNetDevice {
    fn read(&mut self, offset: u32) -> Result<u32, DeviceError> {
        self.check_rx();

        if offset >= BUFFER_OFFSET && offset < BUFFER_OFFSET + NET_BUFFER_SIZE as u32 {
            let idx = (offset - BUFFER_OFFSET) as usize;
            if idx + 4 > NET_BUFFER_SIZE {
                return Ok(0);
            }
            // Read from RX buffer
            let val = u32::from_le_bytes([
                self.rx_buffer[idx],
                self.rx_buffer[idx + 1],
                self.rx_buffer[idx + 2],
                self.rx_buffer[idx + 3],
            ]);
            return Ok(val);
        }

        match offset {
            REG_STATUS => Ok(if self.data_ready { 1 } else { 0 }),
            REG_LENGTH => Ok(self.rx_packet_len), // Read returns RX len
            REG_MAC_LOW => {
                let val = u32::from_le_bytes([self.mac[0], self.mac[1], self.mac[2], self.mac[3]]);
                Ok(val)
            }
            REG_MAC_HIGH => {
                let val = u32::from_le_bytes([self.mac[4], self.mac[5], 0, 0]);
                Ok(val)
            }
            _ => Ok(0),
        }
    }

    fn write(&mut self, offset: u32, val: u32) -> Result<(), DeviceError> {
        if offset >= BUFFER_OFFSET && offset < BUFFER_OFFSET + NET_BUFFER_SIZE as u32 {
            let idx = (offset - BUFFER_OFFSET) as usize;
            if idx + 4 > NET_BUFFER_SIZE {
                return Ok(());
            }
            // Write to TX buffer
            let bytes = val.to_le_bytes();
            self.tx_buffer[idx] = bytes[0];
            self.tx_buffer[idx + 1] = bytes[1];
            self.tx_buffer[idx + 2] = bytes[2];
            self.tx_buffer[idx + 3] = bytes[3];
            return Ok(());
        }

        match offset {
            REG_COMMAND => {
                match val {
                    1 => {
                        // Send
                        let len = self.tx_packet_len as usize;
                        if len > 0 && len <= NET_BUFFER_SIZE {
                            let _ = self.socket.send(&self.tx_buffer[..len]);
                        }
                        Ok(())
                    }
                    2 => {
                        // Recv (Ack data read)
                        self.data_ready = false;
                        self.rx_packet_len = 0;
                        Ok(())
                    }
                    _ => Ok(()),
                }
            }
            REG_LENGTH => {
                self.tx_packet_len = val; // Write sets TX len
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn name(&self) -> &str {
        "virtio-net-simple"
    }

    fn tick(&mut self) -> Result<Option<DeviceInterrupt>, DeviceError> {
        self.check_rx();
        if self.data_ready {
            Ok(Some(DeviceInterrupt {
                device_name: "virtio-net".into(),
                irq_number: 2,
            }))
        } else {
            Ok(None)
        }
    }
}

// Dummy impl for no_std (kernel view)
#[cfg(not(feature = "std"))]
impl Device for SimpleNetDevice {
    fn read(&mut self, _offset: u32) -> Result<u32, DeviceError> {
        Ok(0)
    }
    fn write(&mut self, _offset: u32, _val: u32) -> Result<(), DeviceError> {
        Ok(())
    }
    fn name(&self) -> &str {
        "virtio-net-stub"
    }
    fn tick(&mut self) -> Result<Option<DeviceInterrupt>, DeviceError> {
        Ok(None)
    }
}
