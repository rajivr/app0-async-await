use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use futures_core::stream::Stream;

use crate::result::Result;
use crate::syscalls::{command, subscribe, CallbackData};

const DRIVER_NUM: usize = 3;

mod subscribe_num {
    pub const CALLBACK: usize = 0;
}

mod command_num {
    pub const NUM_BUTTONS: usize = 0;
    pub const ENABLE_INTERRUPT: usize = 1;
    pub const DISABLE_INTERRUPT: usize = 2;
    pub const CURRENT_STATE: usize = 2;
}

static mut BUTTON_CALLBACK_DATA: Option<CallbackData> = None;

static mut BUTTON_WAKER: Option<Waker> = None;

extern "C" fn button_callback(arg0: usize, arg1: usize, arg2: usize, userdata: usize) {
    let cb_data = CallbackData::new(arg0, arg1, arg2, userdata);

    unsafe {
        BUTTON_CALLBACK_DATA = Some(cb_data);

        BUTTON_WAKER.as_ref().map(|w| {
            w.wake_by_ref();
        });
    }
}

pub struct Button;

impl Button {
    pub fn new() -> Button {
        Button
    }

    unsafe fn set_button_waker(cx: &mut Context<'_>) {
        if BUTTON_WAKER.is_none() {
            BUTTON_WAKER = Some(cx.waker().clone());
        }
    }

    pub fn initialize(&self) -> Result<()> {
        unsafe {
            subscribe(
                DRIVER_NUM,
                subscribe_num::CALLBACK,
                button_callback as *const _,
                0,
            )
            .map(|_| ())
        }
    }

    pub fn get_num_buttons(&self) -> Result<usize> {
        unsafe { command(DRIVER_NUM, command_num::NUM_BUTTONS, 0, 0) }
    }

    pub fn enable_button_interrupt(&self, button_num: usize) -> Result<()> {
        unsafe { command(DRIVER_NUM, command_num::ENABLE_INTERRUPT, button_num, 0).map(|_| ()) }
    }

    pub fn disable_button_interrupt(&self, button_num: usize) -> Result<()> {
        unsafe { command(DRIVER_NUM, command_num::DISABLE_INTERRUPT, button_num, 0).map(|_| ()) }
    }

    pub fn get_button_state(&self, button_num: usize) -> Result<ButtonState> {
        unsafe {
            command(DRIVER_NUM, command_num::CURRENT_STATE, button_num, 0).map(|r| {
                if r == 0 {
                    ButtonState::Released
                } else {
                    ButtonState::Pressed
                }
            })
        }
    }
}

impl Stream for Button {
    type Item = ButtonEventData;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        unsafe {
            if let Some(cb_data) = BUTTON_CALLBACK_DATA.take() {
                Button::set_button_waker(cx);

                let button_num = cb_data.get_arg0();
                let button_event_data = if cb_data.get_arg1() == 0 {
                    ButtonEventData::new(button_num, ButtonState::Released)
                } else {
                    ButtonEventData::new(button_num, ButtonState::Pressed)
                };

                Poll::Ready(Some(button_event_data))
            } else {
                Button::set_button_waker(cx);
                Poll::Pending
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct ButtonEventData {
    state: ButtonState,
    num: usize,
}

impl ButtonEventData {
    pub fn new(num: usize, state: ButtonState) -> ButtonEventData {
        ButtonEventData { num, state }
    }

    pub fn get_num(&self) -> usize {
        self.num
    }

    pub fn get_state(&self) -> ButtonState {
        self.state
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum ButtonState {
    Released,
    Pressed,
}
