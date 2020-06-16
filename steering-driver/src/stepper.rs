use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::timer::{CountDown, Periodic};
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::time::Hertz;

use cortex_m::interrupt::Mutex;
use core::cell::Cell;
use crate::mpsc;

pub enum Msg {
    Zero,
    Goto(i32),
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub struct State {
    position: i32,
    action: Action
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum Action {
    Zeroing,
    Ready,
}

pub struct StepperController {
    rx: mpsc::Rx<Msg>,
    state: Mutex<Cell<State>>,
}

impl StepperController {
    pub const fn new() -> StepperController {
        StepperController {
            rx: mpsc::Rx::new(),
            state: Mutex::new(Cell::new(State {
                action: Action::Ready,
                position: 0,
            })),
        }
    }

    pub fn zero(&self) {
        self.rx.sender().send(Msg::Zero);
    }

    pub fn goto(&self, pos: i32) {
        self.rx.sender().send(Msg::Goto(pos));
    }
}

pub struct Stepper<'a, ENA, DIR, PUL, PEND, ALM, LIMR, LIML, TIM> {
    timer: TIM,
    ena: ENA,
    dir: DIR,
    pul: PUL,
    pend: PEND,
    alm: ALM,
    lim_r: LIMR,
    lim_l: LIML,
    pos: i32,
    target: i32,
    ppr: u32,
    cmd: &'a crate::mpsc::Rx<Msg>,
    state: &'a Mutex<Cell<State>>,
}

impl<
        'a,
        ENA: OutputPin,
        DIR: OutputPin,
        PUL: OutputPin,
        PEND: InputPin,
        ALM: InputPin,
        LIMR: InputPin,
        LIML: InputPin,
        TIM: CountDown<Time = Hertz> + Periodic,
    > Stepper<'a, ENA, DIR, PUL, PEND, ALM, LIMR, LIML, TIM>
{
    pub fn new(
        ena: ENA,
        dir: DIR,
        pul: PUL,
        pend: PEND,
        alm: ALM,
        lim_r: LIMR,
        lim_l: LIML,
        timer: TIM,
        ppr: u32,
        controller: &'a StepperController,
    ) -> Self {
        Stepper {
            timer,
            ena,
            dir,
            pul,
            pend,
            alm,
            lim_r,
            lim_l,
            pos: 0,
            target: 0,
            ppr,
            cmd: &controller.rx,
            state: &controller.state,
        }
    }
    /// Blocks forever
    pub fn run(&'a mut self) -> ! {
        loop {
            let cmd = cortex_m::interrupt::free(|cs| {
                self.state.borrow(&cs).set(State {
                    action: Action::Ready,
                    position: self.pos,
                });
                self.cmd.recv_cs(&cs)
            });
            match cmd {
                Some(Msg::Zero) => self.zero(),
                Some(Msg::Goto(p)) => self.target = p,
                None => (),
            }
            if self.pos < self.target {
                self.step_left();
            } else if self.pos > self.target {
                self.step_right();
            }
            else {
                // Make sure we give time to other interrupts
                nb::block!(self.timer.wait());
            }
        }
    }

    fn zero(&mut self) {
        self.ena.set_high();
        while self.lim_l.is_low() {
            self.step_left();
        }
        // Zero position
        self.pos = 0;
        cortex_m::interrupt::free(|cs| {
            self.state.borrow(&cs).set(State {
                action: Action::Ready,
                position: self.pos,
            });
        });
    }

    fn step_left(&mut self) {
        if self.lim_l.is_low() {
            self.dir.set_low();
            self.single_step();
            self.pos -= 1;
        }
    }

    fn step_right(&mut self) {
        if self.lim_r.is_low() {
            self.dir.set_low();
            self.single_step();
            self.pos += 1;
        }
    }

    pub fn set_speed(&mut self, rpm: u32) {
        self.timer.start((2 * self.ppr * rpm / 60).hz());
    }

    fn single_step(&mut self) {
        self.pul.set_high();
        nb::block!(self.timer.wait());
        self.pul.set_low();
        nb::block!(self.timer.wait());
        // Wait for motor to reach position
        while self.pend.is_high() {}
    }
}
