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
- Bounded initial reset handshake (`RST` -> `RST-ACK`) before normal traffic.
- Cumulative acknowledgements and timed retransmission of unacknowledged `DATA` frames.
- Optional EZSP adapters implementing `ezsp::Transmit` and `ezsp::Receive`.

Important behavior details:

- `start(reader, writer, response)` accepts separate async reader and writer values. The caller is
  responsible for opening and configuring the transport and splitting it when necessary. Both
  transport values must be `Send + Unpin` and may borrow from the caller.
- The core crate does not depend on `serialport` or `async-serialport`. Serial ports, sockets,
  in-memory streams, and other transports can be used when they implement the required Tokio I/O
  traits.
- `start(...)` returns transmitter and receiver futures for the caller to poll concurrently on
  an executor compatible with the transport. On native targets, ASH deadlines use a shared
  timer thread.
- The crate does not spawn Tokio tasks internally.
- Dropping every `Handle` clone closes the outbound queue. Both futures end once queued and
  in-flight transmissions finish or the link fails.
- `Handle::send(payload).await` completes when the NCP acknowledges the ASH frame. The inbound
  response channel delivers `DATA` payloads, including any EZSP response.
- During the initial handshake or while the transmit window is full, requests wait in the
  bounded queue. When that queue fills, further calls to `Handle::send` wait for capacity.
  Incoming control frames and retry deadlines remain active.
- The host acknowledges inbound `DATA` after buffering it. A full response channel sets the
  ASH not-ready flag to pause NCP callbacks.
- An exhausted retry budget, transport failure, or unexpected NCP reset ends the session and
  fails queued and in-flight sends with the terminal I/O error. Retry exhaustion returns
  `TimedOut`. Start a fresh ASH session and renegotiate EZSP before sending more commands.
  Commands already written can have an unknown remote outcome.
- Payload type is `heapless::Vec<u8, MAX_PAYLOAD_SIZE>` (`MAX_PAYLOAD_SIZE` defaults to `128`).

Compile-time tunables (via `const_env`):

- `ASHV2_MAX_PAYLOAD_SIZE` (default: `128`)
- `ASHV2_T_RSTACK_MAX_MILLIS` (default: `3200`)
- `ASHV2_TX_K` (default: `5`, range: `1` to `7`)
- `ASHV2_T_RX_ACK_MAX_MILLIS` (default: `3200`, also bounds each transport write)

## Usage

To run the actors as separate Tokio tasks, pass transport types that are both `'static`.
Each returned future's opaque type captures both transport types, so this applies when spawning
either future.

```rust
use ashv2::start;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::channel;

async fn run<R, W>(reader: R, writer: W)
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
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

    // Dropping the final Handle closes the outbound queue. The transmitter
    // finishes outstanding transmissions and signals the receiver to terminate.
    drop(handle);
    transmitter.await.expect("Transmitter task failed");
    receiver.await.expect("Receiver task failed");
}
```

The reader and writer can come from any transport integration. For a bidirectional type that
implements both traits, use that transport's split operation (for example,
`tokio::io::split`) before calling `start(...)`.

To use borrowed I/O, pass values such as `&mut reader` and `&mut writer` to `start` and poll
both actor futures concurrently with the application work in the scope of the borrows.

For graceful shutdown, drop every clone of `Handle`, including handles used through the
optional EZSP transmitter adapter.

## EZSP integration

Enable the `ezsp` feature to get typed EZSP adapters:

```toml
[dependencies]
ashv2 = { version = "19", features = ["ezsp"] }
```

With the feature enabled:

- `ashv2::ezsp::Transmitter` is an alias for
  [`ashv2::Handle`](https://docs.rs/ashv2/latest/ashv2/struct.Handle.html), which implements
  `ezsp::Transmit`.
- `ashv2::ezsp::Receiver` owns the inbound ASHv2 payload receiver and implements
  `ezsp::Receive`. The EZSP layer supplies the currently negotiated protocol version to each
  receive call.

The same payload channel connects the core ASHv2 actor to the EZSP receiver:

```rust
use ashv2::ezsp::{Receiver as EzspReceiver, Transmitter as EzspTransmitter};
use ashv2::start;
use tokio::sync::mpsc::channel;

let (payload_tx, payload_rx) = channel(64);
let (ash_handle, futures) = start(reader, writer, payload_tx);

let ezsp_transmitter: EzspTransmitter = ash_handle;
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
