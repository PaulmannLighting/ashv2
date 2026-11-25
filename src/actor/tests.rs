use std::io::{self, ErrorKind};
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Wake, Waker};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

use tokio::io::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, DuplexStream, duplex, empty, sink, split,
};
use tokio::sync::mpsc::{Receiver, channel};
use tokio::task::{JoinHandle, yield_now};
use tokio::time::timeout;

use crate::code::Code;
use crate::frame::{Ack, Data, Error, Frame, Nak, RstAck};
use crate::protocol::{ControlByte, Stuff, Unstuff};
use crate::types::RawFrame;
use crate::validate::{CRC, Validate};
use crate::{
    Futures, Handle, Payload, SEQ_MASK, T_RSTACK_MAX_MILLIS, T_RX_ACK_MAX_MILLIS, TX_K, VERSION,
    start,
};

const RESPONSE_CAPACITY: usize = 1;
const DUPLEX_CAPACITY: usize = 4096;
const PROMPT: Duration = Duration::from_millis(250);
const ACK_TIMEOUT: Duration = Duration::from_millis(T_RX_ACK_MAX_MILLIS);
const RESET_TIMEOUT: Duration = Duration::from_millis(T_RSTACK_MAX_MILLIS);
const RESET_ATTEMPTS: usize = 6;
const RETRIES: usize = 2;
const INITIAL: u8 = 0;
const NEXT: u8 = 1;
const PAYLOAD: [u8; 3] = [0x7e, 0x7d, 0x1a];

struct Link {
    handle: Handle,
    peer: DuplexStream,
    responses: Receiver<Payload>,
    actors: Futures<JoinHandle<()>, JoinHandle<()>>,
}

struct UnparkThread(Thread);

impl Wake for UnparkThread {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

impl Link {
    fn new() -> Self {
        let (host, peer) = duplex(DUPLEX_CAPACITY);
        let (reader, writer) = split(host);
        let (response, responses) = channel(RESPONSE_CAPACITY);
        let (handle, futures) = start(reader, writer, response);
        let actors = Futures::new(
            tokio::spawn(futures.transmitter),
            tokio::spawn(futures.receiver),
        );
        Self {
            handle,
            peer,
            responses,
            actors,
        }
    }

    async fn connected() -> Self {
        let mut link = Self::new();
        assert!(matches!(link.read(PROMPT).await, Frame::Rst(_)));
        link.reset_ack().await;
        link
    }

    async fn send_frame<I>(&mut self, bytes: I)
    where
        I: IntoIterator<Item = u8>,
    {
        let mut frame = RawFrame::new();
        frame.extend(bytes);
        frame.stuff().expect("stuff peer frame");
        frame
            .push(ControlByte::Flag.into())
            .expect("peer frame flag");
        self.peer.write_all(&frame).await.expect("write peer frame");
    }

    async fn send_control<I>(&mut self, fields: I)
    where
        I: AsRef<[u8]>,
    {
        let fields = fields.as_ref();
        let crc = CRC.checksum(fields).to_be_bytes();
        self.send_frame(fields.iter().copied().chain(crc)).await;
    }

    async fn reset_ack(&mut self) {
        self.send_control([RstAck::HEADER, VERSION, Code::Software as u8])
            .await;
    }

    async fn read(&mut self, limit: Duration) -> Frame {
        timeout(limit, self.read_frame())
            .await
            .expect("frame deadline")
            .expect("host frame")
            .validate()
            .expect("host frame CRC")
    }

    async fn read_frame(&mut self) -> io::Result<Frame> {
        let mut bytes = Vec::new();
        loop {
            let byte = self.peer.read_u8().await?;
            match ControlByte::try_from(byte) {
                Ok(ControlByte::Cancel) => bytes.clear(),
                Ok(ControlByte::Flag) if !bytes.is_empty() => {
                    bytes.unstuff();
                    return Frame::try_from(bytes.drain(..));
                }
                _ => bytes.push(byte),
            }
        }
    }

    async fn write_data(&mut self, frame_num: u8, ack_num: u8) -> JoinHandle<io::Result<()>> {
        let send = self.queue_send();
        self.expect_data(PROMPT, frame_num, ack_num, false).await;
        send
    }

    async fn expect_data(&mut self, limit: Duration, frame_num: u8, ack_num: u8, retransmit: bool) {
        let Frame::Data(data) = self.read(limit).await else {
            panic!("expected DATA")
        };
        assert_eq!(
            (data.frame_num(), data.ack_num(), data.is_retransmission()),
            (frame_num, ack_num, retransmit)
        );
        assert_eq!(*data, make_data(frame_num, ack_num, retransmit));
    }

    fn queue_send(&self) -> JoinHandle<io::Result<()>> {
        let handle = self.handle.clone();
        tokio::spawn(async move { handle.send(payload()).await })
    }

    async fn fill_window(&mut self) -> (u8, Vec<JoinHandle<io::Result<()>>>) {
        let end = u8::try_from(TX_K.get()).expect("three-bit window");
        let mut sends = Vec::with_capacity(TX_K.get());
        for frame in INITIAL..end {
            sends.push(self.write_data(frame, INITIAL).await);
        }
        (end, sends)
    }

    async fn stopped(self) {
        drop(self.handle);
        timeout(PROMPT, self.actors.transmitter)
            .await
            .expect("transmitter stopped")
            .expect("transmitter task");
        timeout(PROMPT, self.actors.receiver)
            .await
            .expect("receiver stopped")
            .expect("receiver task");
    }
}

fn payload() -> Payload {
    PAYLOAD.into_iter().collect()
}

fn make_data(frame: u8, ack: u8, retransmit: bool) -> Data {
    let mut data = Data::new(frame, ack, payload());
    data.set_transmission_fields(ack, retransmit);
    data
}

async fn send_result(send: JoinHandle<io::Result<()>>) -> io::Result<()> {
    timeout(PROMPT, send)
        .await
        .expect("pending send resolved")
        .expect("send task")
}

#[test]
fn start_accepts_send_only_io() {
    fn assert_send<T>(_: T)
    where
        T: Send,
    {
    }

    let reader: Box<dyn AsyncRead + Send + Unpin> = Box::new(empty());
    let writer: Box<dyn AsyncWrite + Send + Unpin> = Box::new(sink());
    let (response, _responses) = channel(RESPONSE_CAPACITY);

    let (_handle, futures) = start(reader, writer, response);
    assert_send(futures);
}

#[tokio::test]
async fn cumulative_ack_reopens_the_window_without_advancing_the_reverse_ack() {
    let mut link = Link::connected().await;
    let (end, sends) = link.fill_window().await;
    let send = link.queue_send();
    assert!(timeout(PROMPT, link.read_frame()).await.is_err());
    assert!(sends.iter().all(|send| !send.is_finished()));
    assert!(!send.is_finished());

    link.send_frame(Ack::new(end, false)).await;
    link.expect_data(PROMPT, end, INITIAL, false).await;
    for send in sends {
        send_result(send).await.expect("cumulative ACK");
    }
    let mut sends = vec![send];
    for frame in NEXT..end {
        sends.push(
            link.write_data(end.wrapping_add(frame) & SEQ_MASK, INITIAL)
                .await,
        );
    }
    link.send_frame(Ack::new(end.wrapping_add(end) & SEQ_MASK, false))
        .await;
    for send in sends {
        send_result(send).await.expect("second cumulative ACK");
    }
    link.stopped().await;
}

#[tokio::test]
async fn dropping_a_send_waiter_keeps_its_frame_in_the_window_until_ack() {
    let mut link = Link::connected().await;
    let (end, mut sends) = link.fill_window().await;
    let abandoned = sends.remove(usize::from(INITIAL));
    abandoned.abort();
    assert!(abandoned.await.expect_err("send cancelled").is_cancelled());

    let queued = link.queue_send();
    assert!(timeout(PROMPT, link.read_frame()).await.is_err());
    link.send_frame(Ack::new(end, false)).await;
    link.expect_data(PROMPT, end, INITIAL, false).await;
    for send in sends {
        send_result(send).await.expect("cumulative ACK");
    }

    link.send_frame(Ack::new(end.wrapping_add(NEXT) & SEQ_MASK, false))
        .await;
    send_result(queued).await.expect("ACK for queued send");
    link.stopped().await;
}

#[tokio::test]
async fn acknowledgement_range_handles_wraparound_and_duplicate_control() {
    let mut link = Link::connected().await;
    for frame in INITIAL..SEQ_MASK {
        let send = link.write_data(frame, INITIAL).await;
        link.send_frame(Ack::new(frame + NEXT, false)).await;
        send_result(send).await.expect("peer ACK");
    }
    let send = link.write_data(SEQ_MASK, INITIAL).await;
    link.send_frame(Ack::new(SEQ_MASK, false)).await;
    assert!(timeout(PROMPT, link.read_frame()).await.is_err());
    // Neither a stale ACK nor an ACK for unsent DATA can release frame 7.
    link.send_frame(Ack::new(SEQ_MASK - NEXT, false)).await;
    assert_eq!(
        link.read(PROMPT).await,
        Frame::Nak(Nak::new(INITIAL, false))
    );
    link.send_frame(Ack::new(NEXT, false)).await;
    assert!(timeout(PROMPT, link.read_frame()).await.is_err());
    link.expect_data(ACK_TIMEOUT + PROMPT, SEQ_MASK, INITIAL, true)
        .await;
    assert!(!send.is_finished());

    link.send_frame(Ack::new(INITIAL, false)).await;
    send_result(send).await.expect("wraparound ACK");
    let send = link.write_data(INITIAL, INITIAL).await;
    link.send_frame(Ack::new(NEXT, false)).await;
    send_result(send).await.expect("peer ACK");
    link.stopped().await;
}

#[tokio::test]
async fn data_ack_is_processed_when_both_application_queues_are_full() {
    let mut link = Link::connected().await;
    let (end, sends) = link.fill_window().await;
    let send = link.queue_send();
    yield_now().await;

    link.send_frame(&make_data(INITIAL, INITIAL, false)).await;
    assert_eq!(link.read(PROMPT).await, Frame::Ack(Ack::new(NEXT, true)));
    assert!(!send.is_finished());

    link.send_frame(&make_data(NEXT, end, false)).await;
    assert_eq!(link.read(PROMPT).await, Frame::Nak(Nak::new(NEXT, true)));
    link.expect_data(PROMPT, end, NEXT, false).await;
    for send in sends {
        send_result(send).await.expect("reverse ACK");
    }
    assert!(!send.is_finished());
    assert_eq!(link.responses.try_recv().expect("first payload"), payload());
    assert_eq!(link.read(PROMPT).await, Frame::Ack(Ack::new(NEXT, false)));

    let host_ack = end.wrapping_add(NEXT) & SEQ_MASK;
    link.send_frame(&make_data(NEXT, host_ack, true)).await;
    assert_eq!(
        link.read(PROMPT).await,
        Frame::Ack(Ack::new(NEXT + NEXT, true))
    );
    send_result(send)
        .await
        .expect("reverse ACK for queued send");
    link.send_frame(&make_data(NEXT, host_ack, true)).await;
    assert_eq!(
        link.read(PROMPT).await,
        Frame::Ack(Ack::new(NEXT + NEXT, true))
    );
    assert_eq!(
        link.responses.try_recv().expect("retried payload"),
        payload()
    );
    assert!(link.responses.try_recv().is_err());
    link.stopped().await;
}

#[tokio::test]
async fn nak_retransmits_the_unacknowledged_suffix_in_order() {
    let mut link = Link::connected().await;
    let (end, mut acknowledged) = link.fill_window().await;
    let ack = if TX_K.get() > 1 { NEXT } else { INITIAL };
    let pending = acknowledged.split_off(usize::from(ack));
    link.send_frame(Nak::new(ack, false)).await;
    for frame in ack..end {
        link.expect_data(PROMPT, frame, INITIAL, true).await;
    }
    for send in acknowledged {
        send_result(send).await.expect("NAK acknowledged prefix");
    }
    assert!(pending.iter().all(|send| !send.is_finished()));
    link.send_frame(Ack::new(end, false)).await;
    for send in pending {
        send_result(send).await.expect("ACK after retransmission");
    }
    link.stopped().await;
}

#[tokio::test]
async fn missing_ack_fails_in_flight_and_queued_sends_and_stops_both_actors() {
    let mut link = Link::connected().await;
    let (end, in_flight) = link.fill_window().await;
    let queued = link.queue_send();
    for _ in 0..RETRIES {
        assert!(timeout(PROMPT, link.read_frame()).await.is_err());
        for frame in INITIAL..end {
            link.expect_data(ACK_TIMEOUT + PROMPT, frame, INITIAL, true)
                .await;
        }
    }
    let error = timeout(ACK_TIMEOUT + PROMPT, queued)
        .await
        .expect("retry limit")
        .expect("send task")
        .expect_err("link failed");
    assert_eq!(error.kind(), ErrorKind::TimedOut);
    for send in in_flight {
        let in_flight_error = send_result(send).await.expect_err("unacknowledged frame");
        assert_eq!(in_flight_error.kind(), error.kind());
        assert_eq!(in_flight_error.to_string(), error.to_string());
    }
    assert_eq!(
        link.handle
            .send(payload())
            .await
            .expect_err("closed link")
            .kind(),
        ErrorKind::BrokenPipe
    );
    link.stopped().await;
}

#[tokio::test]
async fn retransmission_refreshes_the_reverse_ack_and_crc() {
    let mut link = Link::connected().await;
    let send = link.write_data(INITIAL, INITIAL).await;
    link.send_frame(&make_data(INITIAL, INITIAL, false)).await;
    assert_eq!(link.read(PROMPT).await, Frame::Ack(Ack::new(NEXT, true)));
    assert_eq!(link.responses.try_recv().expect("received DATA"), payload());
    assert_eq!(link.read(PROMPT).await, Frame::Ack(Ack::new(NEXT, false)));
    link.expect_data(ACK_TIMEOUT + PROMPT, INITIAL, NEXT, true)
        .await;
    link.send_frame(Ack::new(NEXT, false)).await;
    send_result(send).await.expect("ACK after retransmission");
    link.stopped().await;
}

#[tokio::test]
async fn out_of_order_data_and_duplicates_can_acknowledge_outbound_frames() {
    let mut link = Link::connected().await;
    let send = link.write_data(INITIAL, INITIAL).await;
    link.send_frame(&make_data(INITIAL, INITIAL, false)).await;
    assert_eq!(link.read(PROMPT).await, Frame::Ack(Ack::new(NEXT, true)));
    assert_eq!(
        link.responses.try_recv().expect("inbound payload"),
        payload()
    );
    assert_eq!(link.read(PROMPT).await, Frame::Ack(Ack::new(NEXT, false)));

    link.send_frame(&make_data(NEXT + NEXT, NEXT, false)).await;
    assert_eq!(link.read(PROMPT).await, Frame::Nak(Nak::new(NEXT, false)));
    send_result(send).await.expect("ACK on out-of-order DATA");
    assert!(
        timeout(ACK_TIMEOUT + PROMPT, link.read_frame())
            .await
            .is_err()
    );
    let send = link.write_data(NEXT, NEXT).await;
    link.send_frame(&make_data(INITIAL, NEXT + NEXT, true))
        .await;
    assert_eq!(link.read(PROMPT).await, Frame::Ack(Ack::new(NEXT, false)));
    send_result(send).await.expect("ACK on duplicate DATA");
    assert!(
        timeout(ACK_TIMEOUT + PROMPT, link.read_frame())
            .await
            .is_err()
    );
    assert!(link.responses.try_recv().is_err());
    link.stopped().await;
}

#[tokio::test]
async fn a_full_response_channel_refreshes_and_clears_not_ready() {
    const REFRESH_LIMIT: Duration = Duration::from_secs(1);

    let mut link = Link::connected().await;
    link.send_frame(&make_data(INITIAL, INITIAL, false)).await;
    assert_eq!(link.read(PROMPT).await, Frame::Ack(Ack::new(NEXT, true)));
    assert_eq!(
        link.read(REFRESH_LIMIT).await,
        Frame::Ack(Ack::new(NEXT, true))
    );
    assert_eq!(
        link.responses.try_recv().expect("inbound payload"),
        payload()
    );
    assert_eq!(link.read(PROMPT).await, Frame::Ack(Ack::new(NEXT, false)));
    link.stopped().await;
}

#[tokio::test]
async fn startup_retries_rst_and_ignores_old_frames_before_rstack() {
    let mut link = Link::new();
    assert!(matches!(link.read(PROMPT).await, Frame::Rst(_)));
    let send = link.queue_send();
    link.send_frame(&make_data(INITIAL, INITIAL, false)).await;
    assert!(matches!(
        link.read(RESET_TIMEOUT + PROMPT).await,
        Frame::Rst(_)
    ));
    assert!(link.responses.try_recv().is_err());
    link.reset_ack().await;
    link.expect_data(PROMPT, INITIAL, INITIAL, false).await;
    link.send_frame(Ack::new(NEXT, false)).await;
    send_result(send).await.expect("ACK after handshake retry");
    link.stopped().await;
}

#[test]
fn missing_rstack_fails_the_queued_send_without_a_tokio_runtime() {
    const WATCHDOG_SLACK: Duration = Duration::from_secs(2);

    let attempts = u32::try_from(RESET_ATTEMPTS).expect("reset attempt count");
    let deadline = Instant::now() + RESET_TIMEOUT * attempts + WATCHDOG_SLACK;
    let waker = Waker::from(Arc::new(UnparkThread(thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut test = pin!(async {
        let (host, mut peer) = duplex(DUPLEX_CAPACITY);
        let (reader, writer) = split(host);
        let (response, _responses) = channel(RESPONSE_CAPACITY);
        let (handle, futures) = start(reader, writer, response);
        let ((), (), queued, waiting) = tokio::join!(
            futures.transmitter,
            futures.receiver,
            handle.send(payload()),
            handle.send(payload()),
        );
        assert_eq!(queued.expect_err("no RSTACK").kind(), ErrorKind::TimedOut);
        assert_eq!(
            waiting.expect_err("request queue closed").kind(),
            ErrorKind::BrokenPipe
        );

        let mut wire = Vec::new();
        peer.read_to_end(&mut wire).await.expect("reset frames");
        let flag = u8::from(ControlByte::Flag);
        assert_eq!(
            wire.split_inclusive(|&byte| byte == flag).count(),
            RESET_ATTEMPTS
        );
    });

    while test.as_mut().poll(&mut context).is_pending() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "reset retry budget elapsed");
        thread::park_timeout(remaining);
    }
}

#[tokio::test]
async fn a_blocked_transport_write_resolves_queued_sends() {
    // The Cancel byte fills the wire buffer before the rest of RST is written.
    let (host, _peer) = duplex(RESPONSE_CAPACITY);
    let (reader, writer) = split(host);
    let (response, _responses) = channel(RESPONSE_CAPACITY);
    let (handle, futures) = start(reader, writer, response);
    let transmitter = tokio::spawn(futures.transmitter);
    let receiver = tokio::spawn(futures.receiver);

    let error = timeout(ACK_TIMEOUT + PROMPT, handle.send(payload()))
        .await
        .expect("write deadline")
        .expect_err("blocked transport");
    assert_eq!(error.kind(), ErrorKind::TimedOut);
    timeout(PROMPT, transmitter)
        .await
        .expect("transmitter stopped")
        .expect("transmitter task");
    timeout(PROMPT, receiver)
        .await
        .expect("receiver stopped")
        .expect("receiver task");
}

#[tokio::test]
async fn ncp_error_stops_the_session_with_a_full_window() {
    let mut link = Link::connected().await;
    let (_, mut sends) = link.fill_window().await;
    sends.push(link.queue_send());
    yield_now().await;
    link.send_control([
        Error::HEADER,
        VERSION,
        Code::ExceededMaximumAckTimeoutCount as u8,
    ])
    .await;
    for send in sends {
        assert_eq!(
            send_result(send).await.expect_err("peer failed").kind(),
            ErrorKind::ConnectionReset
        );
    }
    link.stopped().await;
}
