# ashv2

Asynchronous Serial Host protocol, version 2

## Documentation

The official protocol description can be found
at [silabs.com](https://www.silabs.com/documents/public/user-guides/ug101-uart-gateway-protocol-reference.pdf).

## Usage

This library provides the structs `Actor` which implements the ASHv2 protocol.

It is to be initialized with the underlying serial port and the request and response channels used so send and receive
data via the ASHv2 protocol.

```rust
use ashv2::{Actor, BaudRate, FlowControl, open};

#[tokio::main]
async fn main() {
    // Open the serial port connected to the NCP.
    let serial_port = open("/dev/ttyUSB0", BaudRate::RstCts, FlowControl::Hardware)
        .expect("Failed to open serial port");

    // Create the ASHv2 actor, which returns the actor,
    // a proxy to communicate with it, and a receiver for responses.
    let (actor, proxy, mut receiver) = Actor::new(serial_port, 64, 64);
    // Spawn the actor's tasks to handle communication.
    let (_transmitter_task, _receiver_task) = actor.spawn();

    // Send a data frame to the NCP using the proxy.
    // Example: EZSP version command
    let request_data = vec![0x00, 0x00, 0x00, 0x02];
    proxy
        .communicate(request_data.into_iter().collect())
        .await
        .expect("Failed to send request");

    // Receive a response from the NCP.
    if let Some(response) = receiver.recv().await {
        println!("Received response: {response:?}");
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