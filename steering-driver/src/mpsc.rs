use cortex_m::interrupt::{self, CriticalSection, Mutex};
use core::cell::Cell;

pub struct Rx<T> {
    inner: Mutex<Cell<Option<T>>>,
} 

impl<T> Rx<T> {
    pub const fn new() -> Self {
        Rx {
            inner: Mutex::new(Cell::new(None)),
        }
    }

    pub fn recv(&self) -> Option<T> {
        interrupt::free(|cs|
            self.recv_cs(&cs)
        )
    }

    pub fn recv_cs(&self, cs: &CriticalSection) -> Option<T> {
        self.inner.borrow(cs).replace(None)
    }

    pub const fn sender<'a>(&'a self) -> Tx<'a, T> {
        Tx {
            inner: &self.inner
        }
    }
}

#[derive(Copy, Clone)]
pub struct Tx<'a, T> {
    inner: &'a Mutex<Cell<Option<T>>>
}

impl<'a, T> Tx<'a, T> {
    pub fn send(&'a self, value: T) -> Result<(), T> {
        interrupt::free(|cs| {
            match self.inner.borrow(&cs).replace(Some(value)) {
                None => Ok(()),
                Some(x) => Err(self.inner.borrow(&cs).replace(Some(x)).unwrap()),
            }
        })
    }
}
