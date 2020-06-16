use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::timer::{CountDown, Periodic};
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::time::Hertz;

use crate::mpsc;
use core::cell::Cell;
use cortex_m::interrupt::Mutex;

pub enum Msg {
    Zero,
    Goto(i32),
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub struct State {
    position: i32,
    action: Action,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum Action {
    Zeroing,
    Ready,
}

pub struct StepperController {
    target: Mutex<Cell<i32>>
}

impl StepperController {
    pub const fn new() -> StepperController {
        StepperController {
            target: Mutex::new(Cell::new(0))
        }
    }

    pub fn goto(&self, pos: i32) {
        cortex_m::interrupt::free(|cs| {
            self.target.borrow(cs).set(pos);
        });
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
    lim_r_pos: i32,
    pos: i32,
    ppr: u32,
    target: &'a Mutex<Cell<i32>>,
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
        lim_r_pos: i32,
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
            lim_r_pos,
            pos: 0,
            ppr,
            target: &controller.target,
        }
    }
    /// Blocks forever
    pub fn run(&mut self) -> ! {
        loop {
            let target = cortex_m::interrupt::free(|cs| {
                self.target.borrow(cs).get()
            });

            //hprintln!("target: {}, pos: {}", target, self.pos);
            if self.pos > target  {
                self.step_left();
            } else if self.pos < target {
                self.step_right();
            } else {
                // Make sure we give time to other interrupts
                nb::block!(self.timer.wait());
            }
        }
    }

    pub fn zero(&mut self) {
        self.ena.set_low();
        while self.lim_l.is_high() {
            self.step_left();
        }
        // Zero position
        self.pos = 0;
    }

    pub fn step_left(&mut self) {
        if self.lim_l.is_high() {
            self.dir.set_high();
            self.single_step();
            self.pos -= 1;
        }
    }

    pub fn step_right(&mut self) {
        if self.lim_r.is_high() {
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
        //while self.pend.is_high() {}
    }
}
