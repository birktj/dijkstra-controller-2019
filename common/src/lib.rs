#![no_std]

use byteorder::{ByteOrder, LE};
use embedded_hal::serial::{Read, Write};

use core::ops::{Add, Div, Sub, Mul};

mod actuator;
mod adc;

pub use actuator::{Actuator};
pub use adc::{Adc, RefAdc};

pub fn remap<T: Copy + Add<T, Output = T> + Sub<T, Output = T> + Mul<T, Output = T> + Div<T, Output = T>>(val: T, in_l: T, in_h: T, out_l: T, out_h: T) -> T {
    ((val - in_l) * (out_h - out_l) / (in_h - in_l)) + out_l
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum MotorState {
    Idle(u8),
    Fwd(u8),
    Rev(u8),
}

impl MotorState {
    pub fn from_pot(pot: u16) -> Self {
        if pot < 1000 {
            MotorState::Rev(128 - remap(pot as u32, 0, 999, 0, 128) as u8)
        } else if pot < 2000 {
            MotorState::Idle(0)
        } else {
            MotorState::Fwd(remap(pot as u32, 2000, 4096, 0, 255) as u8)
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub struct Frame {
    pub id: u8,
    pub motor_state: MotorState,
    pub motor_direction: u8,
}

impl Frame {
    pub fn write(&self, buf: &mut [u8]) {
        let buf = &mut buf[0..12];
        buf[0] = 0xa3;
        buf[1] = 0xc9;
        buf[2] = 0x3d;
        buf[3] = self.id;
        match self.motor_state {
            MotorState::Idle(p) => {
                buf[4] = 2;
                buf[5] = p;
            }
            MotorState::Fwd(p) => {
                buf[4] = 3;
                buf[5] = p;
            }
            MotorState::Rev(p) => {
                buf[4] = 4;
                buf[5] = p;
            }
        };
        buf[6] = self.motor_direction;

        buf[7] = buf[3];
        buf[8] = buf[4];
        buf[9] = buf[5];
        buf[10] = buf[6];
        buf[11] = 0x65;
    }

    pub fn read(buf: &[u8]) -> Option<Self> {
        let buf = &buf[0..12];
        if &buf[0..3] != &[0xa3, 0xc9, 0x3d] {
            return None;
        }
        let id = buf[3];
        let motor_state = match buf[4] {
            2 => MotorState::Idle(buf[5]),
            3 => MotorState::Fwd(buf[5]),
            4 => MotorState::Rev(buf[5]),
            _ => return None,
        };
        let motor_direction = buf[6];
        if &buf[3..7] != &buf[7..11] {
            return None;
        }
        if buf[11] != 0x65 {
            return None;
        }

        Some(Frame {
            id,
            motor_state,
            motor_direction,
        })
    }

    pub fn send<W: Write<u8>>(&self, writer: &mut W) {
        let mut buf: [u8; 12] = [0; 12];
        self.write(&mut buf);
        for b in &buf {
            nb::block!(writer.write(*b)).ok();
        }
    }
}

pub struct FrameParser {
    circ_buf: [u8; 12],
    i: u8,
}

impl FrameParser {
    pub const fn new() -> FrameParser {
        FrameParser {
            circ_buf: [0; 12],
            i: 0,
        }
    }

    pub fn feed(&mut self, byte: u8) -> Option<Frame> {
        self.circ_buf[self.i as usize] = byte;
        self.i = (self.i + 1) % 12;

        let mut copy = [0; 12];
        for j in 0..12 {
            copy[j] = self.circ_buf[(self.i as usize + j) % 12];
        }

        Frame::read(&copy)
    }

    pub fn recv<R: Read<u8>>(&mut self, reader: &mut R) -> Option<Frame> {
        match reader.read() {
            Ok(x) => self.feed(x),
            _ => None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remap() {
        assert_eq!(remap(5, 0, 10, 0, 100), 50);
        assert_eq!(remap(50, 0, 100, 0, 10), 5);
    }
}
