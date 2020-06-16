use core::cell::Cell;
use crate::actuator::ActuatorController;
use embedded_hal::gpio::OutputPin;

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum PowerState {
    Init,
    Off,
    Starting,
    Idle(u8),
    Fwd(u8),
    Rev(u8),
}


pub struct PowerController {
    target: Cell<PowerState>,
    current: Cell<PowerState>,
}

impl PowerController {
    pub fn new() -> Self {
        Self {
            target: Cell::new(PowerState::Off),
            current: Cell::new(PowerState::Off),
        }
    }

    pub fn start(&self) {
        self.target.set(PowerState::Idle(0));
    }

    pub fn idle(&self, speed: u8) {
        self.target.set(PowerState::Idle(speed));
    }

    pub fn fwd(&self, speed: u8) {
        self.target.set(PowerState::Fwd(speed));
    }

    pub fn rev(&self, speed: u8) {
        self.target.set(PowerState::Rev(speed));
    }
}

pub struct PowerDriver<'a, ON, START> {
    target: &'a Cell<PowerState>,
    current: &'a Cell<PowerState>,
    gear: &'a ActuatorController,
    rev_pos: u16,
    idle_pos: u16,
    fwd_pos: u16,
    throttle: &'a ActuatorController,
    low_speed: u16,
    high_speed: u16,
    on: ON,
    start: START,
}

impl<'a, ON: OutputPin, START: OutputPin> PowerDriver<'a, ON, START> {
    #[coroutine]
    fn run(&'a mut self) {
        self.gear.set_target(self.idle_pos);
        self.throttle.set_target(self.low_speed);
    }
}
