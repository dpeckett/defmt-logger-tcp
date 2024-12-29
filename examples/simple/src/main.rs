use defmt::info;
use std::thread;
use std::time::{Duration, SystemTime};

fn main() {
    thread::spawn(defmt_logger_tcp::run);

    // Allow some time for the logger to attach.
    // Use: `defmt-print -e ./target/debug/simple tcp`
    thread::sleep(Duration::from_secs(10));

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    info!("The current Unix timestamp is: {}", now);
}
