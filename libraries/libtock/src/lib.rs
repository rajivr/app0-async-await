#![no_std]
#![feature(
    asm,
    alloc_error_handler,
    allocator_api,
    core_intrinsics,
    in_band_lifetimes,
    lang_items,
    naked_functions
)]

use heapless::{self, consts, LinearMap};
use linked_list_allocator::{Heap, LockedHeap};

pub mod button;
pub mod console_read;
pub mod console_write;
pub mod entry_point;
pub mod futures;
pub mod lang_items;
pub mod led;
pub mod result;
pub mod syscalls;
pub mod unwind_symbols;

pub use result::Result;

// Even though we do not use a global allocator, when `linked_list_allocator`
// does `extern crate alloc`
#[global_allocator]
static GLOBAL_ALLOC: LockedHeap = LockedHeap::empty();

#[no_mangle]
#[used]
static mut BUTTON_FUTURE_ALLOC: Heap = Heap::empty();

#[no_mangle]
#[used]
static mut CONSOLE_WRITE_FUTURE_ALLOC: Heap = Heap::empty();

#[no_mangle]
#[used]
static mut CONSOLE_READ_FUTURE_ALLOC: Heap = Heap::empty();

// Because `FnvIndexMap` does not have a `const fn` constructor, we have to use
// a `LinearMap`. `usize` here is a pointer to allocated memory.
#[no_mangle]
#[used]
static mut ALLOCED_FUTURE_PTR: LinearMap<usize, futures::AllocedFor, consts::U16> =
    LinearMap(heapless::i::LinearMap::new());
