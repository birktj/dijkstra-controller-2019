use core::cell::{Cell, RefCell};
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::timer::{CountDown, Periodic};
use embedded_hal::adc::{Channel, OneShot};
use embedded_hal::PwmPin;

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


pub struct Actuator<IN1, IN2, PosPin, Lim> {
    in1: IN1,
    in2: IN2,
    pos_pin: PosPin,
    lim: Lim,
    state: State,
    target: Option<u16>,
    position: u16,
}

impl<
        IN1: OutputPin,
        IN2: OutputPin,
        Lim: InputPin,
        PosPin,
        //AdcDev,
        //PosAdc: OneShot<AdcDev, u16, PosPin>,
        //PosPin: Channel,
    > Actuator<IN1, IN2, PosPin, Lim>
{
    pub fn new(
        in1: IN1,
        in2: IN2,
        pos_pin: PosPin,
        lim: Lim,
    ) -> Actuator<IN1, IN2, PosPin, Lim> {
        Actuator {
            in1,
            in2,
            pos_pin,
            lim,
            state: State::Stop,
            target: None,
            position: 0,
        }
    }
    fn update(&mut self) {
        match self.state {
            State::Stop => {
                self.in1.set_low();
                self.in2.set_low();
            }
            State::Fwd => {
                self.in2.set_low();
                self.in1.set_high();
            }
            State::Rev => {
                self.in1.set_low();
                self.in2.set_high();
            }
        }
    }

    pub fn stop(&mut self) {
        if self.state != State::Stop {
            self.state = State::Stop;
            self.update();
        }
        self.target = None;
    }

    pub fn go_fwd(&mut self) {
        if self.state != State::Fwd && self.lim.is_high() {
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

    pub fn goto(&mut self, target: u16) {
        if !self.within(target) || Some(target) == self.target {
            self.target = Some(target);
        }
        else {
            self.stop();
        }
    }

    /*
    pub fn goto<AdcDev, PosAdc: OneShot<AdcDev, u16, PosPin>>(&mut self, pos_adc: &mut PosAdc, target: u16) where PosPin: Channel<AdcDev> {
        self.target = target;

        while self.pos_diff() > 200 {
            self.tick(pos_adc);
        };
    }*/

    fn pos_diff(&self) -> Option<u16> {
        let mi = core::cmp::min(self.position, self.target?);
        let ma = core::cmp::max(self.position, self.target?);
        Some(ma - mi)
    } 

    pub fn tick<AdcDev, PosAdc: OneShot<AdcDev, u16, PosPin>>(&mut self, pos_adc: &mut PosAdc) where PosPin: Channel<AdcDev> {
        self.position = pos_adc.read(&mut self.pos_pin).ok().unwrap();

        if self.lim.is_low() && self.state == State::Fwd {
            self.stop();
            return
        }

        /*
        if self.pos_diff() > 1000 {
            self.set_duty(0);
        }
        else {
            self.set_duty((self.pos_diff()) as u8 / 10 + 50);
        }*/

        if let Some(target) = self.target {
            if self.position + 10 < target {
                self.go_rev();
            }
            /*
            else if self.state == State::Fwd && self.position > self.target + 10 {
                self.go_rev();
            }*/
            else if self.position > target + 10 {
                self.go_fwd();
            }
            /*
            else if self.state == State::Rev && self.position + 10 < self.target {
                self.go_fwd();
            }*/
            else {
                self.stop();
            }
        }
    }

    pub fn position(&self) -> u16 {
        self.position
    }

    pub fn within(&self, pos: u16) -> bool {
        self.position + 50 >= pos && self.position <= pos + 50
    }

    pub fn stopped(&self) -> bool {
        self.target.is_none()
    }
}
