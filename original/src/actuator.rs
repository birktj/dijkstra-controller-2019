use core::cell::{Cell, RefCell};
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::timer::{CountDown, Periodic};
use embedded_hal::PwmPin;
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::time::Hertz;

use crate::adc::Adc;

#[derive(Eq, PartialEq)]
enum State {
    Stop,
    Fwd,
    Rev,
}

pub struct ActuatorController {
    target: Cell<u16>,
    pos: Cell<u16>,
}

impl ActuatorController {
    pub fn new() -> Self {
        ActuatorController {
            target: Cell::new(0),
            pos: Cell::new(0),
        }
    }

    pub fn set_target(&self, target: u16) {
        self.target.set(target);
    }

    pub fn in_position(&self) -> bool {
        self.pos.get() < self.target.get() + 200
            && self.pos.get() + 200 > self.target.get()
    }
}

impl ActuatorController {
    pub fn new_actuator<
        'a,
        IN1: PwmPin<Duty = u16>,
        IN2: PwmPin<Duty = u16>,
        Pos: Adc<Sample = u16>,
        Lim: InputPin,
    >(
        &'a self,
        in1: IN1,
        in2: IN2,
        pos: Pos,
        lim: Lim,
    ) -> Actuator<'a, IN1, IN2, Pos, Lim> {
        Actuator {
            in1,
            in2,
            pos,
            lim,
            duty: 0,
            state: State::Stop,
            target: &self.target,
            position: &self.pos,
        }
    }
}

pub struct Actuator<'a, IN1, IN2, Pos, Lim> {
    in1: IN1,
    in2: IN2,
    pos: Pos,
    lim: Lim,
    duty: u8,
    state: State,
    target: &'a Cell<u16>,
    position: &'a Cell<u16>,
}

impl<
        'a,
        IN1: PwmPin<Duty = u16>,
        IN2: PwmPin<Duty = u16>,
        Pos: Adc<Sample = u16>,
        Lim: InputPin,
    > Actuator<'a, IN1, IN2, Pos, Lim>
{
    fn update(&mut self) {
        match self.state {
            State::Stop => {
                //self.in1.disable();
                //self.in2.disable();
                let in1_duty = self.in1.get_max_duty();
                let in2_duty = self.in2.get_max_duty();
                self.in1.set_duty(in1_duty);
                self.in2.set_duty(in2_duty);
            }
            State::Fwd => {
                //self.in1.disable();
                //self.in2.disable();
                let in2_duty = self.in2.get_max_duty();
                self.in2.set_duty(in2_duty);

                let in1_duty = self.in1.get_max_duty() as u32 * self.duty as u32 / 255;
                self.in1.set_duty(in1_duty as u16);

                self.in1.enable();
                self.in2.enable();
            }
            State::Rev => {
                //self.in1.disable();
                //self.in2.disable();
                let in1_duty = self.in1.get_max_duty();
                self.in1.set_duty(in1_duty);

                let in2_duty = self.in2.get_max_duty() as u32 * self.duty as u32 / 255;
                self.in2.set_duty(in2_duty as u16);

                self.in1.enable();
                self.in2.enable();
            }
        }
    }

    fn stop(&mut self) {
        if self.state != State::Stop {
            self.state = State::Stop;
            self.update();
        }
    }

    pub fn go_fwd(&mut self) {
        if self.state != State::Fwd {
            self.state = State::Fwd;
            self.update();
        }
    }

    pub fn go_rev(&mut self) {
        if self.state != State::Rev {
            self.state = State::Rev;
            self.update();
        }
    }

    fn set_duty(&mut self, duty: u8) {
        if self.duty != duty {
            self.duty = duty;
            self.update();
        }
    }

    #[coroutine]
    pub fn goto(&'a mut self, target: u16) {
        self.target.set(target);

        loop {
            /*
            if self.lim.is_high() && self.state == State::Fwd {
                break;
            }
            */

            self.tick();
            yield;
        };
    }

    #[coroutine]
    pub fn run(&'a mut self) {
        loop {
            self.tick();
            yield;
        };
    }

    fn pos_diff(&self) -> u16 {
        let mi = core::cmp::min(self.position.get(), self.target.get());
        let ma = core::cmp::max(self.position.get(), self.target.get());
        ma - mi
    } 

    pub fn tick(&mut self) {
        /*
        if self.lim.is_high() && self.state == State::Fwd {
            self.stop();
            return
        }*/
        self.position.set(self.pos.read());

        /*
        if self.pos_diff() > 1000 {
            self.set_duty(0);
        }
        else {
            self.set_duty((self.pos_diff()) as u8 / 10 + 50);
        }*/

        if self.position.get() > self.target.get() + 200 {
            self.go_fwd();
        }
        else if self.state == State::Fwd && self.position.get() > self.target.get() + 10 {
            self.go_fwd();
        }
        else if self.position.get() + 200 < self.target.get() {
            self.go_rev();
        }
        else if self.state == State::Rev && self.position.get() + 10 < self.target.get() {
            self.go_rev();
        }
        else {
            self.stop();
        }
    }
}
