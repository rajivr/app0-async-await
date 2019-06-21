use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

use crate::result::{Error, Result, UsizeError};
use crate::syscalls::{allow, command, subscribe, CallbackData};

const DRIVER_NUM: usize = 1;

mod allow_num {
    pub const READ: usize = 2;
}

mod subscribe_num {
    pub const READ: usize = 2;
}

mod command_num {
    pub const READ: usize = 2;
    pub const READ_ABORT: usize = 3;
}

static mut CONSOLE_READ_DATA: Option<CallbackData> = None;

static mut CONSOLE_READ_WAKER: Option<Waker> = None;

extern "C" fn console_read_callback(arg0: usize, arg1: usize, arg2: usize, userdata: usize) {
    let cb_data = CallbackData::new(arg0, arg1, arg2, userdata);

    unsafe {
        CONSOLE_READ_DATA = Some(cb_data);

        CONSOLE_READ_WAKER.as_ref().map(|w| {
            w.wake_by_ref();
        });
    }
}

#[derive(Copy, Clone, PartialEq)]
struct InflightReads {
    pending: usize,
    completed: usize,
}

// Indicates if there is an ongoing read. If the ongoing read was aborted, then
// we go into Aborting and wait for the last callback. Once the read is
// complete, `CONSOLE_READ_STATE` is set to `Nothing` and the future can be
// resolved.
#[derive(Copy, Clone, PartialEq)]
enum ConsoleReadState {
    Ongoing(InflightReads),
    Aborting(InflightReads),
    Nothing,
}

static mut CONSOLE_READ_STATE: ConsoleReadState = ConsoleReadState::Nothing;

// Corresponds to the kernel read buffer
static mut CONSOLE_READ_BUF: [u8; 64] = [0; 64];

pub struct ConsoleRead;

impl ConsoleRead {
    pub fn new() -> ConsoleRead {
        ConsoleRead
    }

    pub fn read_buffer(buf: &mut [u8]) {
        let len = buf.len();
        unsafe {
            buf.copy_from_slice(&CONSOLE_READ_BUF[..len]);
        }
    }

    unsafe fn set_console_read_waker(cx: &mut Context<'_>) {
        if CONSOLE_READ_WAKER.is_none() {
            CONSOLE_READ_WAKER = Some(cx.waker().clone());
        }
    }

    pub fn read(&self, len: usize) -> Result<impl Future<Output = Result<BytesRead>>> {
        unsafe {
            if CONSOLE_READ_STATE != ConsoleReadState::Nothing {
                return Err(Error::EBUSY);
            }

            if len > CONSOLE_READ_BUF.len() {
                return Err(Error::EINVAL);
            }

            // clear previous read
            &CONSOLE_READ_BUF.iter_mut().for_each(|x| *x = 0);

            let _ = allow(
                DRIVER_NUM,
                allow_num::READ,
                &CONSOLE_READ_BUF as *const u8 as *mut u8,
                len,
            )
            .and_then(|_| {
                subscribe(
                    DRIVER_NUM,
                    subscribe_num::READ,
                    console_read_callback as *const _,
                    0,
                )
            })
            .and_then(|_| command(DRIVER_NUM, command_num::READ, len, 0))?;

            CONSOLE_READ_STATE = ConsoleReadState::Ongoing(InflightReads {
                pending: len,
                completed: 0,
            });

            Ok(ConsoleReader)
        }
    }

    // `CONSOLE_READ_STATE` does from `Ongoing(...)` to `Aborting(...)`
    pub fn abort(&self) -> Result<()> {
        unsafe {
            match CONSOLE_READ_STATE {
                ConsoleReadState::Ongoing(InflightReads {
                    pending: rp,
                    completed: rc,
                }) => command(DRIVER_NUM, command_num::READ_ABORT, 0, 0).map(|_| {
                    CONSOLE_READ_STATE = ConsoleReadState::Aborting(InflightReads {
                        pending: rp,
                        completed: rc,
                    });
                    ()
                }),
                ConsoleReadState::Aborting(_) => Err(Error::EBUSY),
                ConsoleReadState::Nothing => Err(Error::EINVAL),
            }
        }
    }
}

// Future returned by ConsoleRead::read
struct ConsoleReader;

impl Future for ConsoleReader {
    type Output = Result<BytesRead>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            if let Some(cb_data) = CONSOLE_READ_DATA.take() {
                let x: UsizeError = cb_data.get_arg0().into();
                match x.0 {
                    Some(e) => {
                        // Callback error
                        CONSOLE_READ_STATE = ConsoleReadState::Nothing;
                        Poll::Ready(Err(e))
                    }
                    None => {
                        // No callback error, roll the state machine
                        let c = CONSOLE_READ_STATE.clone();
                        match c {
                            ConsoleReadState::Ongoing(InflightReads {
                                pending: rp,
                                completed: rc,
                            }) => {
                                let mut rp = rp;
                                let mut rc = rc;

                                rc += cb_data.get_arg1();
                                rp -= cb_data.get_arg1();

                                if rp == 0 {
                                    // Read completed successfully
                                    CONSOLE_READ_STATE = ConsoleReadState::Nothing;
                                    Poll::Ready(Ok(rc))
                                } else {
                                    // Read is still ongoing
                                    CONSOLE_READ_STATE = ConsoleReadState::Ongoing(InflightReads {
                                        pending: rp,
                                        completed: rc,
                                    });
                                    ConsoleRead::set_console_read_waker(cx);
                                    Poll::Pending
                                }
                            }
                            ConsoleReadState::Aborting(InflightReads { completed: rc, .. }) => {
                                let mut rc = rc;

                                rc += cb_data.get_arg1();

                                CONSOLE_READ_STATE = ConsoleReadState::Nothing;
                                Poll::Ready(Ok(rc))
                            }
                            ConsoleReadState::Nothing => {
                                unreachable!();
                            }
                        }
                    }
                }
            } else {
                ConsoleRead::set_console_read_waker(cx);
                Poll::Pending
            }
        }
    }
}

pub type BytesRead = usize;
