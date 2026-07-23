# ASHv2 Architecture

## Scope

This document describes the internal architecture of this crate as currently implemented.
The crate implements the host side of ASHv2 over caller-provided asynchronous byte streams.
The transport is commonly a serial link to an NCP, but the core architecture is independent of
how that stream is opened and configured.

## High-Level Runtime Structure

At runtime, the crate is centered around `start(...)`, which creates two asynchronous
actor futures:

- `Transmitter`: owns the supplied `AsyncWrite` implementation and connection state.
- `Receiver`: owns the supplied `AsyncRead` implementation and inbound frame handling.

`start(reader, writer, response)` takes transport halves that the caller has already opened,
configured, and split. It returns `Handle`, the user-facing send handle, and a `Futures`
container with transmitter and receiver futures that the caller must spawn or poll on their async
runtime. Incoming payloads are pushed to the user-provided response channel.

```mermaid
flowchart TD
    App[Application]
    Transport[Caller-owned transport setup]
    Start[start]
    Handle[Handle]
    Futures[Futures]
    Tx[Transmitter future]
    Rx[Receiver future]
    Reader[AsyncRead implementation]
    Writer[AsyncWrite implementation]
    MsgQ[(tokio mpsc Message queue)]
    RespQ[(tokio mpsc Payload queue)]

    App --> Transport
    Transport --> Reader
    Transport --> Writer
    App -->|reader + writer + response channel| Start
    Reader --> Start
    Writer --> Start
    Start -->|returns| Handle
    Start -->|returns| Futures
    Futures --> Tx
    Futures --> Rx
    App -->|send payload| Handle
    Handle -->|Message::Payload| MsgQ
    MsgQ --> Tx
    Tx -->|write frames| Writer
    Reader -->|read frames| Rx
    Rx -->|inbound payload| RespQ
    RespQ --> App
    Rx -->|ACK/NAK/RST/RST-ACK/ERROR as Message| MsgQ
    Handle -->|Message::Terminate| MsgQ
    Tx -->|sets running=false on exit| Rx
```

## Core Modules and Responsibilities

- `src/actor/*`
  - `start(...)`, internal message bus, future lifecycle, graceful termination signaling.
- `src/actor/receiver/buffer.rs`
  - Receive-side chunk buffering, byte scanning, control-byte handling, unstuffing, and frame parsing.
- `src/actor/transmitter/buffer.rs`
  - Transmit-side frame serialization, byte stuffing, frame termination, and asynchronous writes.
- `src/frame/*`
  - Frame data structures and binary conversion for `DATA`, `ACK`, `NAK`, `RST`, `RST-ACK`, `ERROR`.
- `src/frame/headers/*`
  - Bit-level header composition/parsing for `DATA`, `ACK`, `NAK`.
- `src/protocol/randomization.rs`
  - ASH payload randomization (masking).
- `src/protocol/stuffing.rs`
  - Byte stuffing and unstuffing around control bytes.
- `src/validate.rs`
  - CRC-16-IBM-3740 validation.
- `src/ezsp/*` (feature `ezsp`)
  - Optional adapters from typed EZSP frames to ASHv2 payloads and back.

## Connection and Future Lifecycle

The transmitter is the owner of connection state (`Uninitialized`, `Connected`, `Failed`).
On startup it sends `RST`, waits for `RST-ACK`, and only then handles payload traffic normally.

```mermaid
stateDiagram-v2
    [*] --> Uninitialized
    Uninitialized --> Connected: valid RST-ACK (version=2, in time)
    Uninitialized --> Uninitialized: resend RST / reject messages
    Connected --> Failed: I/O error or inbound RST/ERROR
    Failed --> Uninitialized: reset() sends RST
    Connected --> Connected: DATA/ACK/NAK exchange
```

## Message Flow

### Outbound path (App -> NCP)

1. App calls `Handle::send(payload).await`.
2. `Handle` sends `Message::Payload` into the transmitter queue with a oneshot response channel.
3. Transmitter creates a `DATA` frame:
   - sets frame number (`u8`, masked to 3 bits for modulo-8 behavior),
   - sets current ACK number,
   - masks payload bytes,
   - computes CRC.
4. Transmitter writes via write buffer:
   - convert frame to bytes,
   - stuff reserved control bytes,
   - append `FLAG (0x7E)`,
   - write to the caller-provided `AsyncWrite` implementation.
5. Transmitter stores transmission metadata for ACK/NAK-based completion/retransmission.

### Inbound path (NCP -> App)

1. The caller-provided `AsyncRead` implementation supplies inbound bytes.
2. The receiver buffer retains any unconsumed bytes between reads.
3. Receiver reads bytes until `FLAG`.
4. Receiver handles control bytes (`CANCEL`, `SUBSTITUTE`, `XON`, `XOFF`, `WAKE`) and un-stuffs payload bytes.
5. Parsed bytes are converted into a typed frame and CRC-validated.
6. Receiver behavior by frame type:
   - `DATA`: sequence check, send `ACK` or `NAK`, unmask payload, forward to response channel.
   - `ACK`: notify transmitter to retire sent frames up to ACK number.
   - `NAK`: notify transmitter to retransmit matching sent frame.
   - `RST`, `RST-ACK`, `ERROR`: forward to transmitter for connection-state handling.

```mermaid
sequenceDiagram
    participant A as Application
    participant H as Handle
    participant T as Transmitter
    participant S as Byte Stream
    participant R as Receiver
    participant Q as Response Channel

    A->>H: send(payload)
    H->>T: Message::Payload
    T->>S: DATA(frame, masked payload)
    S->>R: inbound frame bytes
    R->>T: Message::AckSentFrame / NakSentFrame
    R->>Q: unmasked payload
    Q->>A: Payload
```

## Async I/O Path

The core API is generic over separate Tokio `AsyncRead` and `AsyncWrite` implementations. It does
not depend on `serialport` or `async-serialport`, and does not open, configure, or split a serial
port. Those transport-specific operations belong to the calling application.

The receiver buffer reads chunks directly from the supplied reader and retains bytes after a
completed frame for the next read. The transmitter buffer writes fully encoded and stuffed frames
through the supplied writer.

```mermaid
flowchart TD
    Setup[Caller transport setup]
    Reader[AsyncRead]
    Writer[AsyncWrite]
    RxBuffer[Receiver Buffer]
    TxBuffer[Transmitter Buffer]
    Receiver[Receiver future]
    Transmitter[Transmitter future]

    Setup --> Reader
    Setup --> Writer
    Reader --> RxBuffer
    RxBuffer --> Receiver
    Transmitter --> TxBuffer
    TxBuffer --> Writer
```

## Optional EZSP Integration

The `ezsp` feature enables the public `ashv2::ezsp` module and its optional `ezsp` and
`le-stream` dependencies. It does not change the core transport or actor API.

- `ashv2::ezsp::Transmitter` wraps `Handle`, encodes typed EZSP headers and parameters into
  `Payload`, and implements `ezsp::Transmit`.
- `ashv2::ezsp::Receiver` owns the response channel's receiver, decodes each inbound `Payload`
  into a typed EZSP frame, tracks the negotiated EZSP version, and implements `ezsp::Receive`.

```mermaid
flowchart LR
    EzspApi[EZSP API]
    EzspTx[EZSP Transmitter]
    Handle[ASHv2 Handle]
    Actors[ASHv2 actors]
    PayloadQ[(Payload channel)]
    EzspRx[EZSP Receiver]

    EzspApi -->|typed request| EzspTx
    EzspTx -->|encoded Payload| Handle
    Handle --> Actors
    Actors -->|inbound Payload| PayloadQ
    PayloadQ --> EzspRx
    EzspRx -->|typed response| EzspApi
```

## Shutdown Path

`Handle::terminate().await` asks the transmitter to terminate. When the transmitter exits
its main loop, it clears the shared running flag observed by the receiver. The caller owns
the returned futures and is responsible for joining or otherwise observing them on their
async runtime. Transport resource cleanup is also the caller's responsibility. Termination
failures are reported as Tokio message-send errors.

## Frame Types and Purpose

| Frame | Header Pattern | Purpose | Key fields |
|---|---|---|---|
| `DATA` | bit7 = 0 | Carry protocol payload data | frame number (3 bits), retransmit flag, ACK number (3 bits), payload, CRC |
| `ACK` | `0b1000_xxxx` with type bits for ACK | Positive acknowledgement of received data | ACK number, `nRDY`, CRC |
| `NAK` | `0b1010_xxxx` with type bits for NAK | Negative acknowledgement, requests retransmit | ACK number, `nRDY`, CRC |
| `RST` | `0xC0` | Request reset / restart link establishment | fixed header, CRC |
| `RST-ACK` | `0xC1` | Confirm reset and report reset reason/version | protocol version, reset code, CRC |
| `ERROR` | `0xC2` | Signal protocol/link error | protocol version, error code, CRC |

### DATA header bit layout

- bits `6..4`: frame number (`u8`, lower 3 significant bits)
- bit `3`: retransmit flag
- bits `2..0`: ACK number (`u8`, lower 3 significant bits)

### ACK/NAK header bit layout

- ACK base: `0b1000_0000`
- NAK base: `0b1010_0000`
- bit `3`: `nRDY`
- bits `2..0`: ACK number (`u8`, lower 3 significant bits)

## Reliability and Retransmission Model

- Sliding window capacity is `TX_K` (default `5`), stored in a fixed-capacity queue.
- Payload requests are requeued without delay when the sliding window is full.
- Payload sends fail with `ErrorKind::NotConnected` until the initial reset handshake completes.
- Each queued transmission tracks:
  - send time (`Instant`),
  - frame number,
  - retransmit count.
- On inbound `ACK`, matching transmitted frames are retired.
- On inbound `NAK`, matching frame is removed and retransmitted with retransmit flag set.
- Timed-out transmissions are dropped when processing ACK/NAK maintenance.
- After too many retransmissions (`ACK_TIMEOUTS = 4` in current code), transmit returns timeout error.

## CRC Validation

- CRC algorithm: `CRC-16-IBM-3740`.
- CRC is computed over frame bytes excluding the CRC field itself.
- Receiver validates CRC per frame type before semantic handling.
- Invalid CRC in inbound `DATA` triggers `NAK`; invalid control frames are ignored with warning logs.

## Randomization (Masking) in Detail

ASH payload randomization is implemented as XOR masking on payload bytes (not on headers/CRC):

- Generator state defaults:
  - seed: `0x42`
  - feedback mask: `0xB8`
  - flag bit: `0x01`
- For each generated mask byte:
  1. output current `random` value,
  2. shift `random` right by one,
  3. if previous output had flag bit set, XOR shifted value with `0xB8`.

Each payload byte is XORed with one generated mask byte.

Important properties:

- Symmetric transform: applying `mask()` twice restores original bytes.
- Used on send path before CRC computation for `DATA`.
- Used on receive path before delivering payload to the application.

## Byte Stuffing / Unstuffing in Detail

To preserve frame boundaries and control semantics on the serial stream, reserved bytes are escaped.

### Stuffing on transmit

Reserved bytes:

- `0x7D` (`ESCAPE`)
- `0x7E` (`FLAG`)
- `0x11` (`XON`)
- `0x13` (`XOFF`)
- `0x18` (`SUBSTITUTE`)
- `0x1A` (`CANCEL`)

For each reserved byte in frame content:

1. insert `ESCAPE (0x7D)` before it,
2. toggle bit 5 of original byte (`byte ^= 0x20`).

After stuffing all bytes, append final `FLAG (0x7E)` to terminate the frame.

### Unstuffing on receive

The receiver scans bytes until `FLAG`.

- On `ESCAPE`, it removes that byte and marks the next byte for de-escaping.
- The next byte is restored by toggling bit 5 (`byte ^= 0x20`).

Control-byte handling during stream parsing:

- `FLAG (0x7E)`: frame boundary.
- `CANCEL (0x1A)`: clear current buffer and error state.
- `SUBSTITUTE (0x18)`: set error condition; current frame is discarded on next `FLAG`.
- `XON/XOFF`: consumed as flow-control indications (not frame payload data).
- `WAKE (0xFF)`: treated as wake signal when buffer is empty.

## Configuration Knobs

Compile-time environment overridable constants:

- `ASHV2_MAX_PAYLOAD_SIZE` (default `128`)
- `ASHV2_T_RSTACK_MAX_MILLIS` (default `3200`)
- `ASHV2_TX_K` (default `5`)
- `ASHV2_T_RX_ACK_MAX_MILLIS` (default `3200`)

## CI and Quality Gates

GitHub Actions workflow (`.github/workflows/rust.yml`) runs:

- formatting check (`cargo +nightly fmt --check`)
- clippy with warnings denied
- tests (`cargo test --all-features`)
- release build (`cargo build --all-features --release`)
- `cargo vet` supply-chain checks
