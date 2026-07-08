# ashv2

Implementation of Silicon Labs' Asynchronous Serial Host protocol v2 (ASHv2), host side.

## Specification

Silicon Labs publishes the documentation online:

- https://docs.silabs.com/zigbee/latest/uart-gateway-protocol-reference/

## Current implementation status

The crate currently provides:

- Frame parsing/encoding for `DATA`, `ACK`, `NAK`, `RST`, `RST-ACK`, and `ERROR`.
- CRC-16 validation/generation for all supported frame types.
- Byte stuffing/unstuffing and ASH payload randomization (masking/unmasking).
- Async actor futures created with `start(...)`, with caller-owned transmitter/receiver execution.
- Async serial I/O through `async-serialport`, with receiver-side chunking via `ReaderStream`.
- Automatic initial reset handshake (`RST` -> `RST-ACK`) before normal traffic.
- Automatic handling of inbound `ACK`/`NAK` and retransmission of queued `DATA` frames.
- Automatic reset/recovery on protocol errors (`ERROR`, `RST`, and selected I/O failures).

Important behavior details:

- `start(...)` splits the native serial port into async worker, transmitter, and receiver futures, then returns them in
  a named `Futures` container for the caller to spawn or poll.
- The crate does not spawn Tokio tasks internally.
- `Handle::send(payload).await` confirms local transmission attempt (I/O success), not the remote ASH response payload.
- `Handle::send(payload).await` returns `ErrorKind::NotConnected` while the ASH link is not established.
- When the transmit window is full, the transmitter requeues the payload request without delay.
- Incoming `DATA` payloads are delivered through the response channel passed to `start(...)`.
- Payload type is `heapless::Vec<u8, MAX_PAYLOAD_SIZE>` (`MAX_PAYLOAD_SIZE` defaults to `128`).

Compile-time tunables (via `const_env`):

- `ASHV2_MAX_PAYLOAD_SIZE` (default: `128`)
- `ASHV2_T_RSTACK_MAX_MILLIS` (default: `3200`)
- `ASHV2_TX_K` (default: `5`)
- `ASHV2_T_RX_ACK_MAX_MILLIS` (default: `3200`)

## Usage

```rust
use ashv2::{FlowControl, open, start};
use tokio::sync::mpsc::channel;

#[tokio::main]
async fn main() {
    // Open serial port connected to the NCP.
    // Baud rate is derived from flow control by the crate.
    let serial_port = open("/dev/ttyUSB0", FlowControl::Hardware)
        .expect("Failed to open serial port");

    // Channel for inbound ASH DATA payloads from the NCP.
    let (response_tx, mut response_rx) = channel(64);

    // Create ASH actor futures and spawn them on this application's runtime.
    let (handle, futures) = start(serial_port, response_tx);
    let serial_worker = tokio::spawn(futures.serial_worker);
    let transmitter = tokio::spawn(futures.transmitter);
    let receiver = tokio::spawn(futures.receiver);

    // Example EZSP "version" request payload.
    let request_payload = [0x00, 0x00, 0x00, 0x02].into_iter().collect();
    handle
        .send(request_payload)
        .await
        .expect("Failed to transmit request frame");

    // Receive inbound DATA payload from NCP.
    if let Some(response_payload) = response_rx.recv().await {
        println!("Received response payload: {response_payload:?}");
    }

    // Request graceful shutdown, then join the runtime-owned tasks.
    handle
        .terminate()
        .await
        .expect("Failed to request actor termination");
    receiver.await.expect("Receiver task failed");
    transmitter.await.expect("Transmitter task failed");
    let _serial_port = serial_worker.await.expect("Serial worker task failed");
}
```

## Development

The CI workflow currently runs:

- `cargo +nightly fmt --check`
- `cargo clippy --all-features -- -A clippy::multiple_crate_versions -D warnings`
- `cargo test --all-features`
- `cargo build --all-features --release`
- `cargo vet check`

## Legal

This project is free software and is not affiliated with Silicon Labs.

## Credits

Special thanks to Simon Farnsworth, Kevin Reid, and the community at
https://users.rust-lang.org/.
