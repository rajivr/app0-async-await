#![no_std]
#![feature(asm, async_await, generators)]
#![allow(unused_must_use)]

use core::cell::UnsafeCell;
use core::fmt::Write;
use core::future::Future;
use core::str;

use futures_util::{
    future::{join, select, Either},
    stream::StreamExt,
};

use futures_core::future::LocalFutureObj;

use pin_utils::pin_mut;

#[allow(unused_imports)]
use tock;

use tock::{
    button::{Button, ButtonState},
    console_read::ConsoleRead,
    console_write::{ConsoleWrite, ConsoleWriteStr},
    futures::{
        AllocedFor, ButtonFutureAlloc, ConsoleReadFutureAlloc, ConsoleWriteFutureAlloc, FutureBox,
    },
    led::Led,
    syscalls,
};

use embrio_async::embrio_async;
use embrio_executor;

#[used]
#[no_mangle]
pub static mut DATA: [u32; 128] = [0xC0DEF00D; 128];

#[used]
#[no_mangle]
pub static mut BSS: [u32; 64] = [0x0; 64];

#[derive(Debug)]
pub struct Error;

#[embrio_async]
async fn run3() -> Result<(), Error> {
    let button = Button::new();
    let led = Led::new();
    let console_write = ConsoleWrite::new();

    button.enable_button_interrupt(0);
    button.initialize();

    // StreamFuture<tock::button::Button>
    let mut b_fut = button.into_future();

    loop {
        // impl Future
        let cw_fut = console_write
            .write("Hello world\n".as_bytes())
            .map_err(|_| Error)?;

        // FutureBox<impl Future, ConsoleWriteFutureAlloc>
        let cw_fut_box = FutureBox::new(cw_fut, ConsoleWriteFutureAlloc, AllocedFor::ConsoleWrite);

        // LocalFutureObj<'_, Result<usize, tock::result::Error>>
        let cw_local_future_obj = LocalFutureObj::new(cw_fut_box);

        pin_mut!(cw_local_future_obj);
        // Pin<&mut LocalFutureObj<'_, Result<usize, tock::result::Error>>>
        let pinned_cw_local_future_obj = cw_local_future_obj;

        (pinned_cw_local_future_obj.await).map_err(|_| Error)?;

        // FutureBox<StreamFuture<tock::button::Button>, ButtonFutureAlloc>
        let b_fut_box = FutureBox::new(b_fut, ButtonFutureAlloc, AllocedFor::Button);

        // LocalFutureObj<'_, (Option<tock::button::ButtonEventData>, tock::button::Button)>
        let b_local_future_obj = LocalFutureObj::new(b_fut_box);

        pin_mut!(b_local_future_obj);
        // Pin<&mut LocalFutureObj<'_,
        // (Option<tock::button::ButtonEventData>, tock::button::Button)>
        let pinned_b_local_future_obj = b_local_future_obj;

        // (Option<tock::button::ButtonEventData>, tock::button::Button)
        let (b, b_orig) = pinned_b_local_future_obj.await;
        b_fut = b_orig.into_future();

        if let ButtonState::Pressed = (b.ok_or(Error)?).get_state() {
            led.toggle(0);
        }
    }
}

#[embrio_async]
async fn run4() -> Result<(), Error> {
    let console_write = ConsoleWrite::new();
    let console_read = ConsoleRead::new();

    let mut r_buf: [u8; 64] = [0; 64];
    let mut w_buf: [u8; 64] = [0; 64];

    loop {
        // impl Future
        let cw_fut = console_write
            .write("Enter 5 characters: ".as_bytes())
            .map_err(|_| Error)?;

        // FutureBox<impl Future, ConsoleWriteFutureAlloc>
        let cw_fut_box = FutureBox::new(cw_fut, ConsoleWriteFutureAlloc, AllocedFor::ConsoleWrite);

        // LocalFutureObj<'_, Result<usize, tock::result::Error>>
        let cw_local_future_obj = LocalFutureObj::new(cw_fut_box);

        pin_mut!(cw_local_future_obj);
        // Pin<&mut LocalFutureObj<'_, Result<usize, tock::result::Error>>>
        let pinned_cw_local_future_obj = cw_local_future_obj;

        (pinned_cw_local_future_obj.await).map_err(|_| Error)?;

        // impl Future
        let cr_fut = console_read.read(5).map_err(|_| Error)?;

        // FutureBox<impl Future, ConsoleReadFutureAlloc>
        let cr_fut_box = FutureBox::new(cr_fut, ConsoleReadFutureAlloc, AllocedFor::ConsoleRead);

        // LocalFutureObj<'_, Result<usize, tock::result::Error>>
        let cr_local_future_obj = LocalFutureObj::new(cr_fut_box);

        pin_mut!(cr_local_future_obj);
        // Pin<&mut LocalFutureObj<'_, Result<usize, tock::result::Error>>>
        let pinned_cr_local_future_obj = cr_local_future_obj;

        (pinned_cr_local_future_obj.await).map_err(|_| Error)?;

        ConsoleRead::read_buffer(&mut r_buf[..5]);

        let mut w = ConsoleWriteStr::new(&mut w_buf[..]);
        write!(w, "\nReceived: {} \n", str::from_utf8(&r_buf[..5]).unwrap()).unwrap();
        let w_offset = w.get_offset();

        // impl Future
        let cw1_fut = console_write.write(&w_buf[..w_offset]).map_err(|_| Error)?;

        // FutureBox<impl Future, ConsoleWriteFutureAlloc>
        let cw1_fut_box =
            FutureBox::new(cw1_fut, ConsoleWriteFutureAlloc, AllocedFor::ConsoleWrite);

        // LocalFutureObj<'_, Result<usize, tock::result::Error>>
        let cw1_local_future_obj = LocalFutureObj::new(cw1_fut_box);

        pin_mut!(cw1_local_future_obj);
        // Pin<&mut LocalFutureObj<'_, Result<usize, tock::result::Error>>>
        let pinned_cw1_local_future_obj = cw1_local_future_obj;

        (pinned_cw1_local_future_obj.await).map_err(|_| Error)?;
    }
}

#[embrio_async]
async fn run5() -> Result<(), Error> {
    let button = Button::new();
    let console_read = ConsoleRead::new();
    let console_write = ConsoleWrite::new();
    let led = Led::new();

    button.enable_button_interrupt(0);
    button.initialize();

    // StreamFuture<tock::button::Button>
    let mut b_fut = button.into_future();

    // LocalFutureObj<'_, Result<usize, tock::result::Error>>
    let mut maybe_cr_local_future_obj: Option<
        LocalFutureObj<'_, Result<usize, tock::result::Error>>,
    > = None;

    let mut r_buf: [u8; 64] = [0; 64];
    let mut w_buf: [u8; 64] = [0; 64];

    loop {
        // LocalFutureObj<'_, Result<usize, tock::result::Error>>
        let cw_local_future_obj = console_write
            .write("\nEnter 5 characters or press button: ".as_bytes())
            .map(|f| FutureBox::new(f, ConsoleWriteFutureAlloc, AllocedFor::ConsoleWrite))
            .map(|fb| LocalFutureObj::new(fb))
            .map_err(|_| Error)?;

        pin_mut!(cw_local_future_obj);
        // Pin<&mut LocalFutureObj<'_, Result<usize, tock::result::Error>>>
        let pinned_cw_local_future_obj = cw_local_future_obj;
        (pinned_cw_local_future_obj.await).map_err(|_| Error)?;

        // If there is an unresolved `cr_local_future_obj`, from previous
        // iteration of the loop then use it.
        let cr_local_future_obj = if let Some(x) = maybe_cr_local_future_obj.take() {
            x
        } else {
            // LocalFutureObj<'_, Result<usize, tock::result::Error>>
            console_read
                .read(5)
                .map(|f| FutureBox::new(f, ConsoleReadFutureAlloc, AllocedFor::ConsoleRead))
                .map(|fb| LocalFutureObj::new(fb))
                .map_err(|_| Error)?
        };

        // We are passing `b_fut` directly here. Previously, we converted a
        // `b_fut` into a `b_fut_box` and then put it into a `LocalFutureObj` It
        // seems like the The `fut_box` and `local_future_obj` dance only seems
        // to be required when working with `impl Future`. In case of a `b_fut`,
        // we get `StreamFuture<...>`.
        match select(cr_local_future_obj, b_fut).await {
            // (
            //   Result<usize, tock::result::Error>,
            //   StreamFuture<tock::button::Button>
            //  )
            Either::Left((cr_res, b_fut1)) => {
                // console read resolved
                cr_res.map_err(|_| Error)?;

                ConsoleRead::read_buffer(&mut r_buf[..5]);

                let mut w = ConsoleWriteStr::new(&mut w_buf[..]);
                write!(w, "\nReceived: {} \n", str::from_utf8(&r_buf[..5]).unwrap()).unwrap();
                let w_offset = w.get_offset();

                // LocalFutureObj<'_, Result<usize, tock::result::Error>>
                let cw1_local_future_obj = console_write
                    .write(&w_buf[..w_offset])
                    .map(|f| FutureBox::new(f, ConsoleWriteFutureAlloc, AllocedFor::ConsoleWrite))
                    .map(|fb| LocalFutureObj::new(fb))
                    .map_err(|_| Error)?;

                pin_mut!(cw1_local_future_obj);
                // Pin<&mut LocalFutureObj<'_, Result<usize, tock::result::Error>>>
                let pinned_cw1_local_future_obj = cw1_local_future_obj;
                (pinned_cw1_local_future_obj.await).map_err(|_| Error)?;

                // put unresolved button future back for use in the next
                // iteration of the loop
                b_fut = b_fut1;
            }
            // (
            //   (Option<tock::button::ButtonEventData>, tock::button::Button)
            //   LocalFutureObj<'_, Result<usize, tock::result::Error>>
            // )
            Either::Right((b1, cr_local_future_obj1)) => {
                // button resolved
                let (b, b_orig) = b1;

                match (b.ok_or(Error)?).get_state() {
                    ButtonState::Pressed => {
                        led.on(0);

                        // LocalFutureObj<'_, Result<usize, tock::result::Error>>
                        let cw1_local_future_obj = console_write
                            .write("\nReceived button press. Turning ON LED.\n".as_bytes())
                            .map(|f| {
                                FutureBox::new(f, ConsoleWriteFutureAlloc, AllocedFor::ConsoleWrite)
                            })
                            .map(|fb| LocalFutureObj::new(fb))
                            .map_err(|_| Error)?;

                        pin_mut!(cw1_local_future_obj);
                        // Pin<&mut LocalFutureObj<'_, Result<usize, tock::result::Error>>>
                        let pinned_cw1_local_future_obj = cw1_local_future_obj;
                        (pinned_cw1_local_future_obj.await).map_err(|_| Error)?;
                    }
                    ButtonState::Released => {
                        led.off(0);

                        // LocalFutureObj<'_, Result<usize, tock::result::Error>>
                        let cw1_local_future_obj = console_write
                            .write("\nReceived button release. Turning OFF LED.\n".as_bytes())
                            .map(|f| {
                                FutureBox::new(f, ConsoleWriteFutureAlloc, AllocedFor::ConsoleWrite)
                            })
                            .map(|fb| LocalFutureObj::new(fb))
                            .map_err(|_| Error)?;

                        pin_mut!(cw1_local_future_obj);
                        // Pin<&mut LocalFutureObj<'_, Result<usize, tock::result::Error>>>
                        let pinned_cw1_local_future_obj = cw1_local_future_obj;
                        (pinned_cw1_local_future_obj.await).map_err(|_| Error)?;
                    }
                }

                // In case of button, its a stream, when the button future
                // resolves, it also returns original `Button`, so we can create
                // the next `StreamFuture` from it.
                b_fut = b_orig.into_future();

                // put unresolved console write local future object future back
                // for use in the next iteration of the loop
                maybe_cr_local_future_obj = Some(cr_local_future_obj1);
            }
        };
    }
}

#[embrio_async]
async fn run6() -> Result<(), Error> {
    let button = Button::new();
    let console_read = ConsoleRead::new();
    let console_write = ConsoleWrite::new();

    button.enable_button_interrupt(0);
    button.initialize();

    // StreamFuture<tock::button::Button>
    let mut b_fut = button.into_future();

    let mut r_buf: [u8; 64] = [0; 64];
    let mut w_buf: [u8; 64] = [0; 64];

    loop {
        // LocalFutureObj<'_, Result<usize, tock::result::Error>>
        let cw_local_future_obj = console_write
            .write("\nEnter 5 characters and press button: ".as_bytes())
            .map(|f| FutureBox::new(f, ConsoleWriteFutureAlloc, AllocedFor::ConsoleWrite))
            .map(|fb| LocalFutureObj::new(fb))
            .map_err(|_| Error)?;

        pin_mut!(cw_local_future_obj);
        // Pin<&mut LocalFutureObj<'_, Result<usize, tock::result::Error>>>
        let pinned_cw_local_future_obj = cw_local_future_obj;
        (pinned_cw_local_future_obj.await).map_err(|_| Error)?;

        // LocalFutureObj<'_, Result<usize, tock::result::Error>>
        let cr_local_future_obj = console_read
            .read(5)
            .map(|f| FutureBox::new(f, ConsoleReadFutureAlloc, AllocedFor::ConsoleRead))
            .map(|fb| LocalFutureObj::new(fb))
            .map_err(|_| Error)?;

        // Here again, we are not pinning before calling `.await`
        let (cr, (b, b_orig)) = join(cr_local_future_obj, b_fut).await;
        b.ok_or(Error)?;
        b_fut = b_orig.into_future();

        cr.map_err(|_| Error)?;

        ConsoleRead::read_buffer(&mut r_buf[..5]);

        let mut w = ConsoleWriteStr::new(&mut w_buf[..]);
        write!(
            w,
            "\nReceived: {} and button event\n",
            str::from_utf8(&r_buf[..5]).unwrap()
        )
        .unwrap();
        let w_offset = w.get_offset();

        // LocalFutureObj<'_, Result<usize, tock::result::Error>>
        let cw1_local_future_obj = console_write
            .write(&w_buf[..w_offset])
            .map(|f| FutureBox::new(f, ConsoleWriteFutureAlloc, AllocedFor::ConsoleWrite))
            .map(|fb| LocalFutureObj::new(fb))
            .map_err(|_| Error)?;

        pin_mut!(cw1_local_future_obj);
        // Pin<&mut LocalFutureObj<'_, Result<usize, tock::result::Error>>>
        let pinned_cw1_local_future_obj = cw1_local_future_obj;
        (pinned_cw1_local_future_obj.await).map_err(|_| Error)?;
    }
}

#[inline(never)]
fn main() {
    unsafe {
        asm!("bkpt" :::: "volatile");

        // // generate hardfault
        // asm!("
        //      mov r0, #0
        //      ldr r1, [r0, #0]
        //      " :::: "volatile");
    }

    struct RacyCell<T>(UnsafeCell<T>);
    impl<T> RacyCell<T> {
        const fn new(value: T) -> Self {
            RacyCell(UnsafeCell::new(value))
        }
        unsafe fn get_mut_unchecked(&self) -> &mut T {
            &mut *self.0.get()
        }
    }
    unsafe impl<T> Sync for RacyCell<T> {}
    static EXECUTOR: RacyCell<embrio_executor::Executor> =
        RacyCell::new(embrio_executor::Executor::new());

    unsafe {
        EXECUTOR.get_mut_unchecked().block_on(run6());
    }

    unsafe {
        asm!("bkpt" :::: "volatile");
    }

    // Return to the kernel instead!
    syscalls::yieldk();
}
