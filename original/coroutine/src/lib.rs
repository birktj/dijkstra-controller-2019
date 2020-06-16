#![no_std]
#![feature(generator_trait)]

pub use coroutine_macro::coroutine;

#[macro_export]
macro_rules! await_gen {
    ($gen:expr) => ({
        let mut gen = $gen;
        //futures::pin_mut!(gen);
        loop {
            let res = core::ops::Generator::resume(core::pin::Pin::new(&mut gen));
            match res {
                core::ops::GeneratorState::Yielded(x) => yield x,
                core::ops::GeneratorState::Complete(x) => break x,
            }
        }
    })
}

#[macro_export]
macro_rules! await_nb {
    ($e:expr) => ({
        loop {
            match $e {
                Ok(x) => break Ok(x),
                Err(nb::Error::Other(e)) => break Err(e),
                Err(nb::Error::WouldBlock) => yield,
            }
        }
    })
}

#[macro_export]
macro_rules! block {
    ($gen:expr) => ({
        let mut gen = $gen;

        loop {
            let res = core::ops::Generator::resume(core::pin::Pin::new(&mut gen));
            match res {
                core::ops::GeneratorState::Yielded(_) => (),
                core::ops::GeneratorState::Complete(x) => break x,
            }
        }
    })
}


pub trait GeneratorExt {
    fn tick(&mut self);
}

impl<Gen: core::ops::Generator<Yield = (), Return = ()> + Unpin> GeneratorExt for Gen {
    fn tick(&mut self) {
        let res = core::ops::Generator::resume(core::pin::Pin::new(self));
        match res {
            core::ops::GeneratorState::Yielded(_) => (),
            core::ops::GeneratorState::Complete(_) => (),
        }
    }
}

/*
#[macro_export]
macro_export! take {
    ($gen:expr) => ({
        let mut gen = $gen;
        let res = core::ops::generator::resume(core::pin::pin::new(&mut gen));
        match res {
            core::ops::generatorstate::yielded(_) => None,
            core::ops::generatorstate::complete(x) => Some(x),
        }
    })
}
*/

/*
#[macro_export]
macro_rules! block_multiple {
    ($gen:expr),* => ({
        enum State<G> {
            Generator(G),
            Finished(G::Return),
        }

        use State::*;

        let mut states = ($(Generator($gen)),*);

        loop {
            $({
            })*
        }
        
    })
}*/
