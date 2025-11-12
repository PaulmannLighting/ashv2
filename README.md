# ashv2

Asynchronous Serial Host protocol, version 2

## Documentation

The official protocol description can be found
at [silabs.com](https://www.silabs.com/documents/public/user-guides/ug101-uart-gateway-protocol-reference.pdf).

## Usage

This library provides the struct `Transceiver` which implements the ASHv2 protocol.

It is to be initialized with the underlying serial port and the request and response channels used so send and receive
data via the ASHv2 protocol.

```rust
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread::spawn;

use ashv2::Transceiver;
use serialport;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let (request_tx, request_rx) = mpsc::channel(32);
    let (response_tx, mut response_rx) = mpsc::channel(32);

    let serial_port = serialport::new("/dev/ttyUSB0", 115200)
        .open()
        .expect("Failed to open serial port");

    let mut transceiver = Transceiver::new(serial_port, request_rx, response_tx);
    spawn(|| transceiver.run(Arc::new(AtomicBool::new(true))));

    // Example: Sending a request
    let request_data = vec![0x01, 0x02, 0x03];
    request_tx
        .send(request_data.into_iter().collect())
        .await
        .expect("Failed to send request");

    // Example: Receiving a response
    if let Some(response) = response_rx.recv().await {
        println!("Received response: {:?}", response);
    }
}
```

## Legal

This project is free software and is not affiliated with Siliconlabs.

## Contribution guidelines

* Use `cargo fmt`
* Check code with `cargo clippy`

## Credits

* Special thanks to *Simon Farnsworth*, *Kevin Reid* and the rest of the great community
  at [users.rust-lang.org](https://users.rust-lang.org/).