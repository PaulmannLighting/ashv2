use ashv2::{open, BaudRate, Host};
use serialport::FlowControl;

const TTY: &str = "/dev/ttymxc3";

fn main() {
    let serial_port =
        open(TTY, BaudRate::RstCts, FlowControl::Hardware).expect("Failed to open serial port.");
    let mut ashv2 = Host::new(serial_port);
    ashv2.start(None).expect("Could not start ASHv2.");
}
