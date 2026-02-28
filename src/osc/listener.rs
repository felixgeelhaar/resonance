//! OSC listener — UDP socket listener on a dedicated thread.

use std::io;
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use rosc::decoder;

use super::config::OscConfig;
use super::mapping::apply_osc_message;
use crate::tui::external_input::ExternalInputSender;

/// Active OSC listener running on a background thread.
pub struct OscListener {
    stop_flag: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    port: u16,
}

impl OscListener {
    /// Start listening for OSC messages on a UDP port.
    pub fn start(config: &OscConfig, sender: ExternalInputSender) -> io::Result<Self> {
        let addr = format!("127.0.0.1:{}", config.listen_port);
        let socket = UdpSocket::bind(&addr)?;
        // Set a short timeout so we can check the stop flag periodically
        socket.set_read_timeout(Some(std::time::Duration::from_millis(100)))?;

        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_clone = stop_flag.clone();
        let mappings = config.mappings.clone();
        let port = config.listen_port;

        let thread = thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while !stop_clone.load(Ordering::Relaxed) {
                match socket.recv_from(&mut buf) {
                    Ok((size, _addr)) => {
                        if let Ok((_, packet)) = decoder::decode_udp(&buf[..size]) {
                            match packet {
                                rosc::OscPacket::Message(msg) => {
                                    if let Some(event) = apply_osc_message(&msg, &mappings) {
                                        let _ = sender.send(event);
                                    }
                                }
                                rosc::OscPacket::Bundle(bundle) => {
                                    for content in &bundle.content {
                                        if let rosc::OscPacket::Message(msg) = content {
                                            if let Some(event) = apply_osc_message(msg, &mappings) {
                                                let _ = sender.send(event);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        // Timeout — loop and check stop flag
                        continue;
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                        continue;
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        });

        Ok(Self {
            stop_flag,
            thread: Some(thread),
            port,
        })
    }

    /// Get the listening port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Signal the listener to stop.
    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for OscListener {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::osc::config::OscConfig;
    use crate::tui::external_input;

    #[test]
    fn start_and_stop() {
        let config = OscConfig {
            listen_port: 19000, // Use a high port to avoid conflicts
            mappings: Vec::new(),
        };
        let (tx, _rx) = external_input::external_channel();
        let mut listener = OscListener::start(&config, tx).unwrap();
        assert_eq!(listener.port(), 19000);
        listener.stop();
    }

    #[test]
    fn send_and_receive_osc() {
        use rosc::{encoder, OscMessage, OscPacket, OscType};
        use std::net::UdpSocket;

        let config = OscConfig {
            listen_port: 19001,
            mappings: vec![crate::osc::mapping::OscMapping {
                address_pattern: "/play".to_string(),
                target: crate::osc::mapping::OscTarget::PlayStop,
            }],
        };
        let (tx, rx) = external_input::external_channel();
        let mut listener = OscListener::start(&config, tx).unwrap();

        // Send an OSC message
        let msg = OscPacket::Message(OscMessage {
            addr: "/play".to_string(),
            args: vec![],
        });
        let encoded = encoder::encode(&msg).unwrap();
        let sender_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        sender_socket.send_to(&encoded, "127.0.0.1:19001").unwrap();

        // Wait a bit for the listener thread to process
        std::thread::sleep(std::time::Duration::from_millis(200));

        let event = rx.poll();
        assert_eq!(event, Some(external_input::ExternalEvent::PlayStop));

        listener.stop();
    }

    #[test]
    fn bind_failure_on_used_port() {
        let config1 = OscConfig {
            listen_port: 19002,
            mappings: Vec::new(),
        };
        let (tx1, _rx1) = external_input::external_channel();
        let _listener1 = OscListener::start(&config1, tx1).unwrap();

        // Try to bind same port — should fail
        let config2 = OscConfig {
            listen_port: 19002,
            mappings: Vec::new(),
        };
        let (tx2, _rx2) = external_input::external_channel();
        let result = OscListener::start(&config2, tx2);
        assert!(result.is_err());
    }
}
