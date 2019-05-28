use core::task::{RawWaker, RawWakerVTable, Waker};

mod tock;
pub use self::tock::EmbrioWaker;

static EMBRIO_WAKER_RAW_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    |data| unsafe { (*(data as *const EmbrioWaker)).raw_waker() },
    |data| unsafe { (*(data as *const EmbrioWaker)).wake() },
    |data| unsafe { (*(data as *const EmbrioWaker)).wake() },
    |_| (/* Noop */),
);

impl EmbrioWaker {
    pub(crate) fn waker(&'static self) -> Waker {
        unsafe { Waker::from_raw(self.raw_waker()) }
    }

    pub(crate) fn raw_waker(&'static self) -> RawWaker {
        RawWaker::new(
            self as *const _ as *const (),
            &EMBRIO_WAKER_RAW_WAKER_VTABLE,
        )
    }
}
