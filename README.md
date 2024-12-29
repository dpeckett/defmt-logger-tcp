# defmt-logger-tcp

A [defmt](https://defmt.ferrous-systems.com/) logger that sends logs over TCP.

## Usage

```rust
use defmt::info;
use std::thread;

thread::spawn(defmt_logger_tcp::run);
 
info!("Hello, world!");
```

Then you can tail the logs using:

```sh
defmt-print -e ./target/debug/my-app tcp
```

Logs are served via a TCP server listening on `localhost:19021`.