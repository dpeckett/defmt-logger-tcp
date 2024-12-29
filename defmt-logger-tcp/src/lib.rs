//! # A defmt logger that sends logs over TCP.
//!
//! ## Usage
//!
//! ```rust
//! use defmt::info;
//! use std::thread;
//!
//! thread::spawn(|| {
//!     defmt_logger_tcp::init().unwrap();
//! });
//!
//! info!("Hello, world!");
//! ```

use defmt::Encoder;

#[cfg(feature = "std")]
use std::{
    io::{self, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};

static TAKEN: AtomicBool = AtomicBool::new(false);
static PENDING_STREAMS: Mutex<Vec<(TcpStream, Encoder)>> = Mutex::new(Vec::new());
static STREAMS: Mutex<Vec<(TcpStream, Encoder)>> = Mutex::new(Vec::new());

/// Initialize the logger, and start listening for connections on `localhost:19021`.
pub fn init() -> io::Result<()> {
    let listener = TcpListener::bind("localhost:19021")?;

    for stream in listener.incoming() {
        let stream = stream?;

        // Don't block excessively on writes.
        let timeout = Duration::from_millis(100);
        stream.set_write_timeout(Some(timeout))?;

        let mut streams = PENDING_STREAMS.lock().unwrap();
        streams.push((stream, Encoder::new()));
    }

    Ok(())
}

#[defmt::global_logger]
struct Logger;

unsafe impl defmt::Logger for Logger {
    fn acquire() {
        if TAKEN.load(Ordering::Relaxed) {
            panic!("defmt logger taken reentrantly");
        }

        TAKEN.store(true, Ordering::Relaxed);

        on_all_streams(|stream, encoder| {
            let mut result: io::Result<()> = Ok(());
            encoder.start_frame(|bytes| write_stream(stream, bytes, &mut result));
            result
        });
    }

    unsafe fn release() {
        on_all_streams(|stream, encoder| {
            let mut result: io::Result<()> = Ok(());
            encoder.end_frame(|bytes| write_stream(stream, bytes, &mut result));
            result
        });

        // Move pending streams to active streams.
        STREAMS
            .lock()
            .unwrap()
            .extend(PENDING_STREAMS.lock().unwrap().drain(..));

        TAKEN.store(false, Ordering::Relaxed);
    }

    unsafe fn write(bytes: &[u8]) {
        on_all_streams(|stream, encoder| {
            let mut result: io::Result<()> = Ok(());
            encoder.write(bytes, |bytes| write_stream(stream, bytes, &mut result));
            result
        });
    }

    unsafe fn flush() {
        on_all_streams(|stream, _| stream.flush());
    }
}

fn on_all_streams(op: impl Fn(&mut TcpStream, &mut Encoder) -> io::Result<()>) {
    let mut streams = STREAMS.lock().unwrap();

    let mut streams_to_drop: Vec<SocketAddr> = Vec::new();
    for (stream, encoder) in streams.iter_mut() {
        if op(stream, encoder).is_err() {
            streams_to_drop.push(stream.peer_addr().unwrap());
        }
    }

    for stream in streams_to_drop {
        streams.retain(|(s, _)| s.peer_addr().unwrap() != stream);
    }
}

fn write_stream(stream: &mut TcpStream, bytes: &[u8], result: &mut io::Result<()>) {
    if let Err(e) = stream.write_all(bytes) {
        *result = Err(e);
    }
    *result = Ok(());
}
