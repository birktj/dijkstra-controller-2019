use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::timer::{CountDown, Periodic};
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::time::Hertz;

use core::ops::{Generator, GeneratorState};
use core::cell::Cell;

#[derive(Eq, PartialEq, Copy, Clone)]
enum State {
    Init,
    Position(i32),
}

pub struct SteeringController {
    target: Cell<i32>,
    state: Cell<State>,
}

impl SteeringController {
    pub fn new() -> SteeringController {
        SteeringController {
            target: Cell::new(128),
            state: Cell::new(State::Init),
        }
    }
    
    pub fn new_steering<'a,
        ENA: OutputPin,
        DIR: OutputPin,
        PUL: OutputPin,
        PEND: InputPin,
        ALM: InputPin,
        LIMR: InputPin,
        LIML: InputPin,
        TIM: CountDown<Time = Hertz> + Periodic,
    >
    (
        &'a self,
        ena: ENA,
        dir: DIR,
        pul: PUL,
        pend: PEND,
        alm: ALM,
        lim_r: LIMR,
        lim_l: LIML,
        timer: TIM,
        ppr: u32,
    ) -> Steering<ENA, DIR, PUL, PEND, ALM, LIMR, LIML, TIM> {
        Steering {
            timer,
            ena,
            dir,
            pul,
            pend,
            alm,
            lim_r,
            lim_l,
            pos_lim_r: 0,
            pos_lim_l: 0,
            pos: 0,
            ppr,
            target: &self.target,
            state: &self.state,
        }
    }

    pub fn set_position(&self, pos: i32) {
        self.target.set(pos);
    }

    pub fn in_position(&self) -> bool {
        self.state.get() == State::Position(self.target.get())
    }
}

macro_rules! single_step {
    ($self:expr) => ({
        $self.pul.set_high();
        await_nb!($self.timer.wait());
        $self.pul.set_low();
        await_nb!($self.timer.wait());
    })
}

macro_rules! step_right {
    ($self:expr) => ({
        if $self.lim_r.is_low() {
            $self.dir.set_low();
            single_step!($self);
            $self.pos += 1;
        }
    })
}

macro_rules! step_left {
    ($self:expr) => ({
        if $self.lim_r.is_low() {
            $self.dir.set_low();
            single_step!($self);
            $self.pos += 1;
        }
    })
}


pub struct Steering<'a, ENA, DIR, PUL, PEND, ALM, LIMR, LIML, TIM> {
    timer: TIM,
    ena: ENA,
    dir: DIR,
    pul: PUL,
    pend: PEND,
    alm: ALM,
    lim_r: LIMR,
    lim_l: LIML,
    pos_lim_r: i32,
    pos_lim_l: i32,
    pos: i32,
    ppr: u32,
    target: &'a Cell<i32>,
    state: &'a Cell<State>,
}

impl<'a,
        ENA: OutputPin,
        DIR: OutputPin,
        PUL: OutputPin,
        PEND: InputPin,
        ALM: InputPin,
        LIMR: InputPin,
        LIML: InputPin,
        TIM: CountDown<Time = Hertz> + Periodic,
    > Steering<'a, ENA, DIR, PUL, PEND, ALM, LIMR, LIML, TIM>
{
    #[coroutine]
    pub fn run(&'a mut self) {
        self.state.set(State::Init);
        self.ena.set_high();
        while self.lim_l.is_low() {
            step_left!(self);
        }
        self.pos_lim_l = self.pos;

        while self.lim_r.is_low() {
            step_right!(self);
        }
        self.pos_lim_r = self.pos;

        self.state.set(State::Position(self.pos));

        loop {
            if self.pos < self.target.get() {
                step_left!(self);
            }
            else if self.pos > self.target.get() {
                step_right!(self);
            }
            self.state.set(State::Position(self.pos));
            yield;
        }
    }

    /*
    #[coroutine]
    pub fn init<'b>(&'a mut self) where 'a: 'b {
        self.ena.set_high();
        while self.lim_l.is_low() {
            await_gen!(self.step_left());
        }
        self.pos_lim_l = self.pos;
        while self.lim_r.is_low() {
            await_gen!(self.step_right());
        }
        self.pos_lim_r = self.pos;
    }*/

    /*
    //#[coroutine]
    //fn step_left<'b>(&'b mut self) where 'b: 'b {
    fn step_left<'b>(&'b mut self) -> impl core::ops::Generator<Yield = (), Return = ()> + 'a where 'b: 'a {
        move || {
        if self.lim_l.is_low() {
            self.dir.set_low();
            await_gen!(self.single_step());
            self.pos -= 1;
        }
        }
    }

    //#[coroutine]
    fn step_right<'b>(&'b mut self) -> impl core::ops::Generator<Yield = (), Return = ()> + 'a where 'b: 'a {
        move || {
        if self.lim_r.is_low() {
            self.dir.set_low();
            await_gen!(self.single_step());
            self.pos += 1;
        }
        }
    }
    */

    pub fn set_speed(&mut self, rpm: u32) {
        self.timer.start((2 * self.ppr * rpm / 60).hz());
    }

    /*
    //#[coroutine]
    pub fn single_step<'b>(&'b mut self) -> impl core::ops::Generator<Yield = (), Return = ()> + 'a where 'b: 'a {
        move || {
        self.pul.set_high();
        await_nb!(self.timer.wait());
        self.pul.set_low();
        await_nb!(self.timer.wait());
    }
    }*/
}
