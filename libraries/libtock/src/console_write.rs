use core::fmt;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

use crate::result::{Error, Result, UsizeError};
use crate::syscalls::{allow, command, subscribe, CallbackData};

const DRIVER_NUM: usize = 1;

mod allow_num {
    pub const WRITE: usize = 1;
}

mod subscribe_num {
    pub const WRITE: usize = 1;
}

mod command_num {
    pub const WRITE: usize = 1;
}

static mut CONSOLE_WRITE_DATA: Option<CallbackData> = None;

static mut CONSOLE_WRITE_WAKER: Option<Waker> = None;

extern "C" fn console_write_callback(arg0: usize, arg1: usize, arg2: usize, userdata: usize) {
    let cb_data = CallbackData::new(arg0, arg1, arg2, userdata);

    unsafe {
        CONSOLE_WRITE_DATA = Some(cb_data);

        CONSOLE_WRITE_WAKER.as_ref().map(|w| {
            w.wake_by_ref();
        });
    }
}

#[derive(Copy, Clone, PartialEq)]
struct InflightWrites {
    pending: usize,
    completed: usize,
}

// Indicates if there is an ongoing write. Once the write is complete,
// `CONSOLE_WRITE_STATE` is set to `Nothing` and the future can be resolved.
#[derive(Copy, Clone, PartialEq)]
enum ConsoleWriteState {
    Ongoing(InflightWrites),
    Nothing,
}

static mut CONSOLE_WRITE_STATE: ConsoleWriteState = ConsoleWriteState::Nothing;

// Corresponds to the kernel write buffer
static mut CONSOLE_WRITE_BUF: [u8; 64] = [0; 64];

pub struct ConsoleWrite;

impl ConsoleWrite {
    pub fn new() -> ConsoleWrite {
        ConsoleWrite
    }

    unsafe fn set_console_write_waker(cx: &mut Context<'_>) {
        if CONSOLE_WRITE_WAKER.is_none() {
            CONSOLE_WRITE_WAKER = Some(cx.waker().clone());
        }
    }

    pub fn write(&self, s: &[u8]) -> Result<impl Future<Output = Result<BytesWritten>>> {
        unsafe {
            if CONSOLE_WRITE_STATE != ConsoleWriteState::Nothing {
                return Err(Error::EBUSY);
            }

            if s.len() > CONSOLE_WRITE_BUF.len() {
                return Err(Error::EINVAL);
            }

            self.clear_console_write_buf();

            &CONSOLE_WRITE_BUF[..s.len()].copy_from_slice(s);

            let _ = allow(
                DRIVER_NUM,
                allow_num::WRITE,
                &CONSOLE_WRITE_BUF as *const u8 as *mut u8,
                s.len(),
            )
            .and_then(|_| {
                subscribe(
                    DRIVER_NUM,
                    subscribe_num::WRITE,
                    console_write_callback as *const _,
                    0,
                )
            })
            .and_then(|_| command(DRIVER_NUM, command_num::WRITE, s.len(), 0))
            .map_err(|e| {
                self.clear_console_write_buf();
                e
            })?;

            CONSOLE_WRITE_STATE = ConsoleWriteState::Ongoing(InflightWrites {
                pending: s.len(),
                completed: 0,
            });

            Ok(ConsoleWriter)
        }
    }

    fn clear_console_write_buf(&self) {
        unsafe {
            &CONSOLE_WRITE_BUF.iter_mut().for_each(|x| *x = 0);
        }
    }
}

// Future returned by ConsoleWrite::write
struct ConsoleWriter;

impl Future for ConsoleWriter {
    type Output = Result<BytesWritten>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            if let Some(cb_data) = CONSOLE_WRITE_DATA.take() {
                let x: UsizeError = cb_data.get_arg0().into();
                match x.0 {
                    Some(e) => {
                        // Callback error
                        CONSOLE_WRITE_STATE = ConsoleWriteState::Nothing;
                        Poll::Ready(Err(e))
                    }
                    None => {
                        // No callback error, roll the state machine
                        let c = CONSOLE_WRITE_STATE.clone();
                        match c {
                            ConsoleWriteState::Ongoing(InflightWrites {
                                pending: wp,
                                completed: wc,
                            }) => {
                                // No callback error
                                let mut wp = wp;
                                let mut wc = wc;

                                wc += cb_data.get_arg0();
                                wp -= cb_data.get_arg0();

                                if wp == 0 {
                                    // Write completed successfully
                                    CONSOLE_WRITE_STATE = ConsoleWriteState::Nothing;
                                    Poll::Ready(Ok(wc))
                                } else {
                                    // Write is still congoing
                                    CONSOLE_WRITE_STATE =
                                        ConsoleWriteState::Ongoing(InflightWrites {
                                            pending: wp,
                                            completed: wc,
                                        });
                                    ConsoleWrite::set_console_write_waker(cx);
                                    Poll::Pending
                                }
                            }
                            ConsoleWriteState::Nothing => {
                                unreachable!();
                            }
                        }
                    }
                }
            } else {
                ConsoleWrite::set_console_write_waker(cx);
                Poll::Pending
            }
        }
    }
}

pub type BytesWritten = usize;

pub struct ConsoleWriteStr<'a> {
    buf: &'a mut [u8],
    offset: usize,
}

impl<'a> ConsoleWriteStr<'a> {
    pub fn new(buf: &'a mut [u8]) -> ConsoleWriteStr<'a> {
        ConsoleWriteStr {
            buf: buf,
            offset: 0,
        }
    }

    pub fn get_offset(&self) -> usize {
        self.offset
    }
}

// https://stackoverflow.com/a/39491059
impl<'a> fmt::Write for ConsoleWriteStr<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // &[u8]
        let bytes = s.as_bytes();

        // Skip over already-copied data
        let remainder = &mut self.buf[self.offset..];
        // Check if there is space remaining (return error instead of panicking)
        if remainder.len() < bytes.len() {
            return Err(core::fmt::Error);
        }
        // Make the two slices the same length
        let remainder = &mut remainder[..bytes.len()];
        // Copy
        remainder.copy_from_slice(bytes);

        // Update offset to avoid overwriting
        self.offset += bytes.len();

        Ok(())
    }
}
