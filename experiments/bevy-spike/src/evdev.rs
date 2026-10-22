//! Minimal direct evdev reader, for the packaged display test's exit.
//!
//! The launcher starts the app and hands over the panel; the app must always
//! be able to give it back. `--seconds` and the watchdog cover that, and this
//! reader adds "press Guide (or Start) to quit" so a session does not have to
//! run to its limit. It is a trimmed copy of `game/src/evdev.rs`, where the
//! same node-probing logic is exercised on this device: the pad is an
//! ordinary `/dev/input/event*` node carrying `BTN_*` codes; `BRICK_EVDEV`
//! overrides which node is opened.

/// One decoded `struct input_event`.
#[derive(Debug, Clone, Copy)]
pub struct EvdevEvent {
    pub kind: u16,
    pub code: u16,
    pub value: i32,
}

pub const EV_KEY: u16 = 1;
pub const EV_ABS: u16 = 3;

#[cfg(target_os = "linux")]
mod imp {
    use std::fs::File;
    use std::io::Read;
    use std::os::unix::io::AsRawFd;
    use std::path::PathBuf;
    use std::sync::mpsc::{Receiver, channel};
    use std::thread;

    use super::{EV_ABS, EV_KEY, EvdevEvent};

    /// `EVIOCGBIT(EV_KEY, 96)`: `_IOC(_IOC_READ, 'E', 0x20 + 1, 96)`.
    /// 96 bytes covers `KEY_MAX` (0x2ff) bits, which is where the `BTN_` range
    /// lives. Spelled out because `libc` does not export `_IOC` everywhere.
    const EVIOCGBIT_KEY: libc::c_ulong = 0x8060_4521;
    /// Buttons that identify a gamepad: BTN_SOUTH (0x130) and BTN_MODE (0x13c).
    const IDENTITY_BUTTONS: [u16; 2] = [0x130, 0x13c];
    /// `struct input_event` on 64-bit Linux: 16 bytes of timeval, then
    /// `__u16 type`, `__u16 code`, `__s32 value`.
    const RECORD: usize = 24;

    fn has_button(fd: i32, code: u16) -> bool {
        let mut bits = [0u8; 96];
        // SAFETY: `bits` is 96 bytes, which is exactly what the request encodes.
        let result = unsafe { libc::ioctl(fd, EVIOCGBIT_KEY, bits.as_mut_ptr()) };
        if result < 0 {
            return false;
        }
        bits[code as usize / 8] & (1 << (code % 8)) != 0
    }

    fn find_pad() -> Option<PathBuf> {
        for index in 0..12 {
            let path = PathBuf::from(format!("/dev/input/event{index}"));
            let Ok(file) = File::open(&path) else {
                continue;
            };
            if IDENTITY_BUTTONS
                .iter()
                .all(|code| has_button(file.as_raw_fd(), *code))
            {
                return Some(path);
            }
        }
        None
    }

    pub fn start() -> Option<(PathBuf, Receiver<EvdevEvent>)> {
        let path = match std::env::var("BRICK_EVDEV") {
            Ok(explicit) => PathBuf::from(explicit),
            Err(_) => find_pad()?,
        };
        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(error) => {
                println!("evdev: cannot open {}: {error}", path.display());
                return None;
            }
        };
        let (sender, receiver) = channel();
        let node = path.clone();
        thread::Builder::new()
            .name("evdev".into())
            .spawn(move || {
                let mut buffer = [0u8; RECORD];
                loop {
                    if file.read_exact(&mut buffer).is_err() {
                        println!("evdev: {} closed", node.display());
                        return;
                    }
                    let kind = u16::from_le_bytes([buffer[16], buffer[17]]);
                    let code = u16::from_le_bytes([buffer[18], buffer[19]]);
                    let value =
                        i32::from_le_bytes([buffer[20], buffer[21], buffer[22], buffer[23]]);
                    if (kind == EV_KEY || kind == EV_ABS)
                        && sender.send(EvdevEvent { kind, code, value }).is_err()
                    {
                        return;
                    }
                }
            })
            .ok()?;
        Some((path, receiver))
    }
}

#[cfg(not(target_os = "linux"))]
mod imp {
    use std::path::PathBuf;
    use std::sync::mpsc::Receiver;

    use super::EvdevEvent;

    /// A desktop has no evdev gamepad node.
    pub fn start() -> Option<(PathBuf, Receiver<EvdevEvent>)> {
        None
    }
}

pub use imp::start;
