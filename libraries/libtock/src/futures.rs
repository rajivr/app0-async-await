use core::alloc::Layout;
use core::future::Future;
use core::ptr::NonNull;

use allocator_api::{Alloc, AllocErr, Box};
use futures_core::future::UnsafeFutureObj;

use crate::ALLOCED_FUTURE_PTR;
use crate::BUTTON_FUTURE_ALLOC;
use crate::CONSOLE_READ_FUTURE_ALLOC;
use crate::CONSOLE_WRITE_FUTURE_ALLOC;

#[derive(Debug)]
pub enum AllocedFor {
    Button,
    ConsoleWrite,
    ConsoleRead,
}

pub struct ButtonFutureAlloc;

unsafe impl Alloc for ButtonFutureAlloc {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        BUTTON_FUTURE_ALLOC.alloc(layout)
    }

    unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        BUTTON_FUTURE_ALLOC.dealloc(ptr, layout)
    }
}

pub struct ConsoleWriteFutureAlloc;

unsafe impl Alloc for ConsoleWriteFutureAlloc {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        CONSOLE_WRITE_FUTURE_ALLOC.alloc(layout)
    }

    unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        CONSOLE_WRITE_FUTURE_ALLOC.dealloc(ptr, layout)
    }
}

pub struct ConsoleReadFutureAlloc;

unsafe impl Alloc for ConsoleReadFutureAlloc {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        CONSOLE_READ_FUTURE_ALLOC.alloc(layout)
    }

    unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        CONSOLE_READ_FUTURE_ALLOC.dealloc(ptr, layout)
    }
}

pub struct FutureBox<F, A>
where
    A: Alloc,
{
    b: Box<F, A>,
    alloced_for: AllocedFor,
}

impl<F, A> FutureBox<F, A>
where
    A: Alloc,
{
    pub fn new(f: F, a: A, alloced_for: AllocedFor) -> FutureBox<F, A> {
        FutureBox {
            b: Box::new_in(f, a),
            alloced_for: alloced_for,
        }
    }
}

unsafe impl<'a, T, F, A> UnsafeFutureObj<'a, T> for FutureBox<F, A>
where
    F: Future<Output = T> + 'a,
    A: Alloc + 'a,
{
    fn into_raw(self) -> *mut (dyn Future<Output = T> + 'a) {
        let ptr = Box::into_raw(self.b as Box<dyn Future<Output = T>, A>) as *mut _;

        match self.alloced_for {
            AllocedFor::Button => unsafe {
                ALLOCED_FUTURE_PTR
                    .insert(ptr as *mut usize as usize, self.alloced_for)
                    .unwrap()
            },
            AllocedFor::ConsoleWrite => unsafe {
                ALLOCED_FUTURE_PTR
                    .insert(ptr as *mut usize as usize, self.alloced_for)
                    .unwrap()
            },
            AllocedFor::ConsoleRead => unsafe {
                ALLOCED_FUTURE_PTR
                    .insert(ptr as *mut usize as usize, self.alloced_for)
                    .unwrap()
            },
        };

        ptr
    }

    unsafe fn drop(ptr: *mut (dyn Future<Output = T> + 'a)) {
        match ALLOCED_FUTURE_PTR
            .get(&(ptr as *mut usize as usize))
            .unwrap()
        {
            AllocedFor::Button => {
                drop(Box::from_raw_in(ptr as *mut F, ButtonFutureAlloc));
            }
            AllocedFor::ConsoleWrite => {
                drop(Box::from_raw_in(ptr as *mut F, ConsoleWriteFutureAlloc));
            }
            AllocedFor::ConsoleRead => {
                drop(Box::from_raw_in(ptr as *mut F, ConsoleReadFutureAlloc));
            }
        };
    }
}
