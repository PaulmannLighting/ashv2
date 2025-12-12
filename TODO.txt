# TODOs

## Architectural Improvements

- [ ] Implement Actor model for better concurrency handling.
    - [ ] Use `Transmitter` thread which receives `Frame`s through a channel.
    - [ ] Use `Receiver` thread to receive frames and forward ACKs, NAKs, etc. to `Transmitter`.
    - [ ] Wrap `Transmitter` and `Receiver` in a higher-level `Transceiver` struct.