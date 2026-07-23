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
- Transport-independent async I/O over caller-provided `tokio::io::AsyncRead` and
  `tokio::io::AsyncWrite` implementations.
- Automatic initial reset handshake (`RST` -> `RST-ACK`) before normal traffic.
- Automatic handling of inbound `ACK`/`NAK` and retransmission of queued `DATA` frames.
- Automatic reset/recovery on protocol errors (`ERROR`, `RST`, and selected I/O failures).
- Optional EZSP adapters implementing `ezsp::Transmit` and `ezsp::Receive`.

Important behavior details:

- `start(reader, writer, response)` accepts separate async reader and writer values. The caller is
  responsible for opening and configuring the transport and splitting it when necessary.
- The core crate does not depend on `serialport` or `async-serialport`. Serial ports, sockets,
  in-memory streams, and other transports can be used when they implement the required Tokio I/O
  traits.
- `start(...)` returns transmitter and receiver futures in a named `Futures` container for the
  caller to spawn or poll.
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
use ashv2::start;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::channel;

async fn run<R, W>(reader: R, writer: W)
where
    R: AsyncRead + Send + Sync + Unpin + 'static,
    W: AsyncWrite + Send + Sync + Unpin + 'static,
{
    // Channel for inbound ASH DATA payloads from the NCP.
    let (response_tx, mut response_rx) = channel(64);

    // Create ASH actor futures from the transport halves and spawn them on the
    // application's runtime.
    let (handle, futures) = start(reader, writer, response_tx);
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

    // Request actor shutdown. The application owns the spawned tasks.
    handle
        .terminate()
        .await
        .expect("Failed to request actor termination");
    transmitter.await.expect("Transmitter task failed");
    receiver.await.expect("Receiver task failed");
}
```

The reader and writer can come from any transport integration. For a bidirectional type that
implements both traits, use that transport's split operation (for example,
`tokio::io::split`) before calling `start(...)`.

## EZSP integration

Enable the `ezsp` feature to get typed EZSP adapters:

```toml
[dependencies]
ashv2 = { version = "10", features = ["ezsp"] }
```

With the feature enabled:

- `ashv2::ezsp::Transmitter` wraps an [`ashv2::Handle`](https://docs.rs/ashv2/latest/ashv2/struct.Handle.html)
  and implements `ezsp::Transmit`.
- `ashv2::ezsp::Receiver` owns the inbound ASHv2 payload receiver and implements
  `ezsp::Receive`.

The same payload channel connects the core ASHv2 actor to the EZSP receiver:

```rust
use ashv2::ezsp::{Receiver as EzspReceiver, Transmitter as EzspTransmitter};
use ashv2::start;
use tokio::sync::mpsc::channel;

let (payload_tx, payload_rx) = channel(64);
let (ash_handle, futures) = start(reader, writer, payload_tx);

let ezsp_transmitter = EzspTransmitter::new(ash_handle);
let ezsp_receiver = EzspReceiver::new(payload_rx);

// Spawn or otherwise poll futures.transmitter and futures.receiver.
// Pass ezsp_transmitter and ezsp_receiver to the EZSP API.
```

Without the feature, the EZSP dependency and adapter module are not compiled.

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
