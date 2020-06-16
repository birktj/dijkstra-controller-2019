use core::future::Future;
use core::task::Poll;

fn raw_waker() -> core::task::RawWaker {
    core::task::RawWaker::new(0 as *const (), &VTABLE)
}

static VTABLE: core::task::RawWakerVTable =
    core::task::RawWakerVTable::new(|_| raw_waker(), |_| {}, |_| {}, |_| {});

fn block_on<F: Future>(mut future: F) -> F::Output {
    let waker = unsafe { core::task::Waker::from_raw(raw_waker()) };
    let mut context = core::task::Context::from_waker(&waker);

    futures::pin_mut!(future);

    loop {
        match Future::poll(future.as_mut(), &mut context) {
            Poll::Ready(x) => return x,
            Poll::Pending => (),
        }
    }
}

pub trait BlockingFuture: Future {
    fn wait(self) -> Self::Output;
}

impl<F: Future> BlockingFuture for F {
    fn wait(mut self) -> Self::Output {
        block_on(self)
    }
}

pub trait Pollable {
    type Output;
    fn poll(self) -> Poll<Self::Output>;
}

impl<T, E> Pollable for nb::Result<T, E> {
    type Output = Result<T, E>;

    fn poll(self) -> Poll<Self::Output> {
        match self {
            Ok(a) => Poll::Ready(Ok(a)),
            Err(nb::Error::WouldBlock) => Poll::Pending,
            Err(nb::Error::Other(e)) => Poll::Ready(Err(e)),
        }
    }
}
