use embedded_hal::digital::InputPin;

const MASK: u32    = 0xfff00fff;
const RISING: u32  = 0x00000fff;
const FALLING: u32 = 0xfff00000;
const HIGH: u32    = 0xffffffff;
const LOW: u32     = 0x00000000;

pub struct Button<Pin: InputPin> {
    pin: Pin,
    state: u32,
}

impl<Pin: InputPin> Button<Pin> {
    pub fn new(pin: Pin) -> Self {
        Button {
            pin: pin,
            state: 0,
        }
    }

    pub fn tick(&mut self) {
        self.state = (self.state << 1) | (self.pin.is_high() as u32);
    }

    pub fn is_rising(&mut self) -> bool {
        if (self.state & MASK) == RISING {
            self.state = 0xff_ff_ff_ff;
            true
        }
        else {
            false
        }
    }

    pub fn is_falling(&mut self) -> bool {
        if (self.state & MASK) == FALLING {
            self.state = 0x00_00_00_00;
            true
        }
        else {
            false
        }
    }

    pub fn is_high(&self) -> bool {
        self.state == HIGH
    }

    pub fn is_low(&self) -> bool {
        self.state == LOW
    }
}
