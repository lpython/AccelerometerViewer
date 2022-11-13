//! # bevy_serial
//!
//! `bevy_serial` is a plugin to add non-blocking serial communication to bevy. This plugin is based on [`mio-serial`](https://github.com/berkowski/mio-serial)
//! that can realize non-blocking high-performance I/O.
//!
//! Reading and writing from/to serial port is realized via bevy's event system. Each serial port is handled via port
//! name or a unique label you choose. These event handlers are added to the following stage to minimize the frame delay.
//!
//! - Reading: `CoreStage::PreUpdate`
//! - Writing: `CoreStage::PostUpdate`
//!
//! ## Usage
//!
//! ### Simple Example
//!
//! Here is a simple example:
//!
//! ```rust
//! use bevy::prelude::*;
//! use bevy_serial::{SerialPlugin, SerialReadEvent, SerialWriteEvent};
//!
//! // to write data to serial port periodically
//! struct SerialWriteTimer(Timer);
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(MinimalPlugins)
//!         // simply specify port name and baud rate for `SerialPlugin`
//!         .add_plugin(SerialPlugin::new("COM5", 115200))
//!         // to write data to serial port periodically (every 1 second)
//!         .insert_resource(SerialWriteTimer(Timer::from_seconds(1.0, true)))
//!         // reading and writing from/to serial port is achieved via bevy's event system
//!         .add_system(read_serial)
//!         .add_system(write_serial)
//!         .run();
//! }
//!
//! // reading event for serial port
//! fn read_serial(mut ev_serial: EventReader<SerialReadEvent>) {
//!     // you can get label of the port and received data buffer from `SerialReadEvent`
//!     for SerialReadEvent(label, buffer) in ev_serial.iter() {
//!         let s = String::from_utf8(buffer.clone()).unwrap();
//!         println!("received packet from {}: {}", label, s);
//!     }
//! }
//!
//! // writing event for serial port
//! fn write_serial(
//!     mut ev_serial: EventWriter<SerialWriteEvent>,
//!     mut timer: ResMut<SerialWriteTimer>,
//!     time: Res<Time>,
//! ) {
//!     if timer.0.tick(time.delta()).just_finished() {
//!         // you can write to serial port via `SerialWriteEvent` with label and buffer to write
//!         let buffer = b"Hello, bevy!";
//!         ev_serial.send(SerialWriteEvent("COM5".to_string(), buffer.to_vec()));
//!     }
//! }
//! ```
//!
//! ### Multiple Serial Ports with Additional Settings
//!
//! You can add multiple serial ports with additional settings.
//!
//! ```rust
//! use bevy::prelude::*;
//! use bevy_serial::{
//!     DataBits, FlowControl, Parity, SerialPlugin, SerialReadEvent, SerialSetting, SerialWriteEvent,
//!     StopBits,
//! };
//! use std::time::Duration;
//!
//! // to write data to serial port periodically
//! struct SerialWriteTimer(Timer);
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(MinimalPlugins)
//!         // you can specify various configurations for multiple serial ports by this way
//!         .add_plugin(SerialPlugin {
//!             settings: vec![SerialSetting {
//!                 label: Some("my_serial".to_string()),
//!                 port_name: "COM5".to_string(),
//!                 baud_rate: 115200,
//!                 data_bits: DataBits::Eight,
//!                 flow_control: FlowControl::None,
//!                 parity: Parity::None,
//!                 stop_bits: StopBits::One,
//!                 timeout: Duration::from_millis(0),
//!             }],
//!         })
//!         // to write data to serial port periodically (every 1 second)
//!         .insert_resource(SerialWriteTimer(Timer::from_seconds(1.0, true)))
//!         // reading and writing from/to serial port is achieved via bevy's event system
//!         .add_system(read_serial)
//!         .add_system(write_serial)
//!         .run();
//! }
//!
//! // reading event for serial port
//! fn read_serial(mut ev_serial: EventReader<SerialReadEvent>) {
//!     // you can get label of the port and received data buffer from `SerialReadEvent`
//!     for SerialReadEvent(label, buffer) in ev_serial.iter() {
//!         let s = String::from_utf8(buffer.clone()).unwrap();
//!         println!("read packet from {}: {}", label, s);
//!     }
//! }
//!
//! // writing event for serial port
//! fn write_serial(
//!     mut ev_serial: EventWriter<SerialWriteEvent>,
//!     mut timer: ResMut<SerialWriteTimer>,
//!     time: Res<Time>,
//! ) {
//!     if timer.0.tick(time.delta()).just_finished() {
//!         // you can write to serial port via `SerialWriteEvent` with label and buffer to write
//!         let buffer = b"Hello, bevy!";
//!         ev_serial.send(SerialWriteEvent("my_serial".to_string(), buffer.to_vec()));
//!     }
//! }
//! ```
//!
//! ## Supported Versions
//!
//! | bevy | bevy_serial |
//! | ---- | ----------- |
//! | 0.6  | 0.2         |
//! | 0.5  | 0.1         |
//!
//! ## License
//!
//! Dual-licensed under either
//!
//! - MIT
//! - Apache 2.0


pub use mio_serial::{DataBits, FlowControl, Parity, StopBits};

use bevy::app::{App, CoreStage, EventReader, EventWriter, Plugin};
use bevy::ecs::system::{Res, ResMut};
use mio::{Events, Interest, Poll, Token};
use mio_serial::SerialStream;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::sync::Mutex;
use std::time::Duration;

/// Plugin that can be added to Bevy
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialPlugin {
    pub settings: Vec<SerialSetting>,
}

impl SerialPlugin {
    pub fn new(port_name: &str, baud_rate: u32) -> Self {
        Self {
            settings: vec![SerialSetting {
                port_name: port_name.to_string(),
                baud_rate,
                ..Default::default()
            }],
        }
    }
}

/// Settings for users to initialize this plugin
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialSetting {
    /// The intuitive name for this serial port
    pub label: Option<String>,
    /// The port name, usually the device path
    pub port_name: String,
    /// The baud rate in symbols-per-second
    pub baud_rate: u32,
    /// Number of bits used to represent a character sent on the line
    pub data_bits: DataBits,
    /// The type of signalling to use for controlling data transfer
    pub flow_control: FlowControl,
    /// The type of parity to use for error checking
    pub parity: Parity,
    /// Number of bits to use to signal the end of a character
    pub stop_bits: StopBits,
    /// Amount of time to wait to receive data before timing out
    pub timeout: Duration,
}

impl Default for SerialSetting {
    fn default() -> Self {
        Self {
            label: None,
            port_name: "".to_string(),
            baud_rate: 115200,
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::from_millis(0),
        }
    }
}

/// Bevy's event type to read serial port
pub struct SerialReadEvent(pub String, pub Vec<u8>);

/// Bevy's event type to read serial port
pub struct SerialWriteEvent(pub String, pub Vec<u8>);

/// Serial struct that is used internally for this crate
#[derive(Debug)]
struct SerialStreamLabeled {
    stream: SerialStream,
    label: String,
    connected: bool,
}

/// Module scope global singleton to store serial ports
static SERIALS: OnceCell<Vec<Mutex<SerialStreamLabeled>>> = OnceCell::new();

/// Component to get an index of serial port based on the label
struct Indices(HashMap<String, usize>);

/// The size of read buffer for one read system call
const DEFAULT_READ_BUFFER_LEN: usize = 2048;

impl Plugin for SerialPlugin {
    fn build(&self, app: &mut App) {
        let poll = Poll::new().unwrap();
        let events = Events::with_capacity(self.settings.len());
        let mut serials: Vec<Mutex<SerialStreamLabeled>> = vec![];
        let mut indices = Indices(HashMap::new());

        for (i, setting) in self.settings.iter().enumerate() {
            // create serial port builder from `serialport` crate
            let port_builder = serialport::new(&setting.port_name, setting.baud_rate)
                .data_bits(setting.data_bits)
                .flow_control(setting.flow_control)
                .parity(setting.parity)
                .stop_bits(setting.stop_bits)
                .timeout(setting.timeout);

            // create `mio_serial::SerailStream` from `seriaport` builder
            let mut stream = SerialStream::open(&port_builder).unwrap_or_else(|e| {
                panic!("Failed to open serial port {}\n{:?}", setting.port_name, e);
            });

            // token index is same as index of vec
            poll.registry()
                .register(&mut stream, Token(i), Interest::READABLE)
                .unwrap_or_else(|e| {
                    panic!("Failed to register stream to poll : {:?}", e);
                });

            // if label is set, use label as a nickname of serial
            // if not, use `port_name` as a nickname
            let label = if let Some(label) = &setting.label {
                label.clone()
            } else {
                setting.port_name.clone()
            };

            // store indices and serials
            indices.0.insert(label.clone(), i);
            serials.push(Mutex::new(SerialStreamLabeled {
                stream,
                label,
                connected: true,
            }));
        }

        // set to global variables lazily
        SERIALS.set(serials).unwrap_or_else(|e| {
            panic!("Failed to set SerialStream to global variable: {:?}", e);
        });

        app.insert_resource(poll)
            .insert_resource(events)
            .insert_resource(indices)
            .add_event::<SerialReadEvent>()
            .add_event::<SerialWriteEvent>()
            .add_system_to_stage(CoreStage::PreUpdate, read_serial)
            .add_system_to_stage(CoreStage::PostUpdate, write_serial);
    }
}

/// Poll serial read event with `Poll` in `mio` crate.
/// If any data has come to serial, `SerialReadEvent` is sent to the system subscribing it.
fn read_serial(
    mut ev_receive_serial: EventWriter<SerialReadEvent>,
    mut poll: ResMut<Poll>,
    mut events: ResMut<Events>,
    indices: Res<Indices>,
) {
    if !indices.0.is_empty() {
        // poll serial read event (should timeout not to block other systems)
        poll.poll(&mut events, Some(Duration::from_micros(1)))
            .unwrap_or_else(|e| {
                panic!("Failed to poll events: {:?}", e);
            });

        // if events have occurred, send `SerialReadEvent` with serial labels and read data buffer
        for event in events.iter() {
            // get serial instance based on the token index
            let serials = SERIALS.get().expect("SERIALS are not initialized");
            let serial_mtx = serials
                .get(event.token().0) // token index is same as index of vec
                .expect("SERIALS are not initialized");

            if event.is_readable() {
                let mut buffer = vec![0_u8; DEFAULT_READ_BUFFER_LEN];
                let mut bytes_read = 0;
                loop {
                    // try to get lock of mutex and send data to event
                    if let Ok(mut serial) = serial_mtx.lock() {
                        if serial.connected {
                            match serial.stream.read(&mut buffer[bytes_read..]) {
                                Ok(0) => {
                                    eprintln!("read connection closed");
                                    serial.connected = false;
                                    break;
                                }
                                // read data successfully
                                // if buffer is full, maybe there is more data to read
                                Ok(n) => {
                                    bytes_read += n;
                                    if bytes_read == buffer.len() {
                                        buffer.resize(buffer.len() + DEFAULT_READ_BUFFER_LEN, 0);
                                    }
                                }
                                // would block indicates no more data to read
                                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                                    let label = serial.label.clone();
                                    let buffer = buffer.drain(..bytes_read).collect();
                                    ev_receive_serial.send(SerialReadEvent(label, buffer));
                                    break;
                                }
                                // if interrupted, we should continue readings
                                Err(ref e) if e.kind() == ErrorKind::Interrupted => {
                                    continue;
                                }
                                // other errors are fatal
                                Err(e) => {
                                    eprintln!("Failed to read serial port {}: {}", serial.label, e);
                                }
                            }
                        } else {
                            eprintln!("{} connection has closed", serial.label);
                        }
                    }
                }
            }
        }
    }
}

/// Write bytes to serial port.
/// The bytes are sent via `SerialWriteEvent` with label of serial port.
fn write_serial(mut ev_write_serial: EventReader<SerialWriteEvent>, indices: Res<Indices>) {
    if !indices.0.is_empty() {
        for SerialWriteEvent(label, buffer) in ev_write_serial.iter() {
            // get index of label
            let &serial_index = indices
                .0
                .get(label)
                .expect(format!("Label {} is not exist", label).as_str());
            let serials = SERIALS.get().expect("SERIALS are not initialized");
            let serial_mtx = serials
                .get(serial_index)
                .expect("SERIALS are not initialized");

            // write buffered data to serial
            let mut bytes_wrote = 0;
            loop {
                // try to get lock of mutex and send data to event
                if let Ok(mut serial) = serial_mtx.lock() {
                    if serial.connected {
                        // write the entire buffered data in a single system call
                        match serial.stream.write(&buffer[bytes_wrote..]) {
                            // error if returned len is less than expected (same as `io::Write::write_all` does)
                            Ok(n) if n < buffer.len() => {
                                eprintln!(
                                    "write size error {} / {}",
                                    n,
                                    buffer.len() - bytes_wrote
                                );
                                bytes_wrote += n;
                            }
                            // wrote queued data successfully
                            Ok(_) => {
                                bytes_wrote += buffer.len();
                            }
                            // would block indicates that this port is not ready so try again
                            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {}
                            // if interrupted, we should try again
                            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                            // other errors are fatal
                            Err(e) => {
                                eprintln!("Failed to write serial port {}: {}", serial.label, e);
                            }
                        }
                    } else {
                        eprintln!("{} connection has closed", serial.label);
                    }

                    if bytes_wrote == buffer.len() {
                        break;
                    } else {
                        continue;
                    }
                }
            }
        }
    }
}
