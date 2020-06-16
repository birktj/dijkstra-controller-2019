#![no_std]
#![no_main]
#![allow(deprecated)]

extern crate panic_halt;

#[macro_use]
extern crate cortex_m_semihosting;

use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::timer::{CountDown, Periodic};

use cortex_m_rt::entry;

use stm32f1::stm32f103::interrupt;
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::pac;
use stm32f1xx_hal::time::Hertz;
use stm32f1xx_hal::timer::Timer;
use stm32f1xx_hal::serial::{Event, Serial, Tx, Rx};
use stm32f1xx_hal::pac::USART1;
use stm32f1xx_hal::adc::Adc;

use cortex_m::interrupt::{Mutex};
use core::cell::{Cell, RefCell};

mod button;
use button::Button;

use common::*;

//mod stepper;
//use stepper::*;

//mod mpsc;
//mod communication;

/* *****************
 * Controller pinout
 * *****************
 * 
 * left_pot: pa0
 * right_pot: pa1
 * 
 * btn_1: pa2 // Switch mode
 * btn_2: pa3
 * btn_3: pa4
 * btn_4: pa5
 * btn_5: pa6
 * btn_6: pa7
 * 
 * serial: pa9 + pa10
 */

enum Mode {
    MotorControl,
    DirectionControl
}

impl Mode {
    fn switch(&mut self) {
        match self {
            Mode::MotorControl => *self = Mode::DirectionControl,
            Mode::DirectionControl => *self = Mode::MotorControl,
        }
    }
}

fn linearize(x: u16) -> u16 {
    if x < 450 {
        common::remap(x as u32, 0, 439, 0, 2047) as u16
    }
    else {
        common::remap(x as u32, 450, 4096, 2048, 4096) as u16
    }
}



#[entry]
fn main() -> ! {
    // Get access to the core peripherals from the cortex-m crate
    let cp = cortex_m::Peripherals::take().unwrap();
    // Get access to the device specific peripherals from the peripheral access crate
    let dp = pac::Peripherals::take().unwrap();

    // Take ownership over the raw flash and rcc devices and convert them into the corresponding
    // HAL structs
    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();
    let mut afio = dp.AFIO.constrain(&mut rcc.apb2);

    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);
    let mut gpiob = dp.GPIOB.split(&mut rcc.apb2);
    let mut gpioc = dp.GPIOC.split(&mut rcc.apb2);

    let mut serial_l = {
        let pin_tx = gpioa.pa9.into_alternate_push_pull(&mut gpioa.crh);
        let pin_rx = gpioa.pa10;
        Serial::usart1(dp.USART1, (pin_tx, pin_rx), &mut afio.mapr, 9_600.bps(), clocks, &mut rcc.apb2)
    };

    let mut serial_r = {
        let pin_tx = gpiob.pb10.into_alternate_push_pull(&mut gpiob.crh);
        let pin_rx = gpiob.pb11;
        Serial::usart3(dp.USART3, (pin_tx, pin_rx), &mut afio.mapr, 9_600.bps(), clocks, &mut rcc.apb1)
    };

    let mut clock = Timer::syst(cp.SYST, 10.hz(), clocks);

    let mut adc = Adc::adc1(dp.ADC1, &mut rcc.apb2);

    let mut left_pot = gpioa.pa0.into_analog(&mut gpioa.crl);
    let mut mid_pot = gpioa.pa1.into_analog(&mut gpioa.crl);
    let mut right_pot = gpioa.pa2.into_analog(&mut gpioa.crl);

    let mut btn_1 = Button::new(gpioa.pa3.into_pull_up_input(&mut gpioa.crl));
    let mut btn_2 = Button::new(gpioa.pa4.into_pull_up_input(&mut gpioa.crl));
    let mut btn_3 = Button::new(gpioa.pa5.into_pull_up_input(&mut gpioa.crl));
    let mut btn_4 = Button::new(gpioa.pa6.into_pull_up_input(&mut gpioa.crl));
    let mut btn_5 = Button::new(gpioa.pa7.into_pull_up_input(&mut gpioa.crl));
    //let mut btn_6 = Button::new(gpioa.pa8.into_pull_up_input(&mut gpioa.crh));
    
    let mut led_1 = gpioa.pa11.into_push_pull_output(&mut gpioa.crh);
    let mut led_2 = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
    let mut led_3 = gpioa.pa15.into_push_pull_output(&mut gpioa.crh);
    let mut led_4 = gpiob.pb3.into_push_pull_output(&mut gpiob.crl);
    let mut led_5 = gpiob.pb4.into_push_pull_output(&mut gpiob.crl);
    let mut led_6 = gpiob.pb5.into_push_pull_output(&mut gpiob.crl);
    let mut led_7 = gpiob.pb6.into_push_pull_output(&mut gpiob.crl);
    let mut led_8 = gpiob.pb7.into_push_pull_output(&mut gpiob.crl);

    let (mut tx_l, mut rx_l) = serial_l.split();
    let (mut tx_r, mut rx_r) = serial_r.split();

    let mut mode = Mode::MotorControl;

    loop {
        if btn_1.is_falling() {
            mode.switch();
        }

        btn_1.tick();
        btn_2.tick();
        btn_3.tick();
        btn_4.tick();
        btn_5.tick();


        /*
        if btn_1.is_rising() {
            let l_pot: u16 = adc.read(&mut left_pot).unwrap();
            let m_pot: u16 = adc.read(&mut mid_pot).unwrap();
            let r_pot: u16 = adc.read(&mut right_pot).unwrap();

            hprintln!("l: {}, l_lin: {}, m: {}, r: {}, r_lin: {}", l_pot, linearize(l_pot), m_pot, 4096 - r_pot, 4096 - linearize(4096 - r_pot));
        }*/

        // Send state
        match clock.wait() {
            Ok(()) => {
                let l_pot: u16 = adc.read(&mut left_pot).unwrap();
                let m_pot: u16 = adc.read(&mut mid_pot).unwrap();
                let r_pot: u16 = adc.read(&mut right_pot).unwrap();

                let l_motor_state = MotorState::from_pot(linearize(l_pot));
                let r_motor_state = MotorState::from_pot(4096 - linearize(4096 - r_pot));
                let motor_direction = (m_pot >> 4) as u8;

                match l_motor_state {
                    MotorState::Idle(_) => led_2.set_high(),
                    _ => led_2.set_low(),
                };

                match r_motor_state {
                    MotorState::Idle(_) => led_7.set_high(),
                    _ => led_7.set_low(),
                };

                let left_frame = Frame {
                    id: 1,
                    motor_state: l_motor_state,
                    motor_direction,
                };

                let right_frame = Frame {
                    id: 2,
                    motor_state: r_motor_state,
                    motor_direction,
                };

                let mut buf = [0; 12];
                left_frame.write(&mut buf);
                //assert_eq!(Frame::read(&buf), Some(left_frame));

                //hprintln!("{:?}", left_frame);

                left_frame.send(&mut tx_l);
                right_frame.send(&mut tx_l);
                //frame.send(&mut tx_r);

                //hprintln!("l: {}, m: {}, r: {}", l_pot, m_pot, r_pot);
                //let motor_direction = common;
                /*
                match mode {
                    Mode::MotorControl => {
                        let l_power = adc.read(&mut left_pot).unwrap();
                        let r_power = adc.read(&mut right_pot).unwrap();

                        let frame_left = Frame {
                            motor_state: MotorState::from_pot(l_power),
                            motor_direction: ,
                        };
                        let frame_right = Frame {
                            motor_state: MotorState::from_pot(r_power),
                            motor_direction: MotorDirection::zero(),
                        };

                        frame_left.send(&mut tx_l);
                        frame_right.send(&mut tx_r);
                    }
                    Mode::DirectionControl => {
                        let dir   = adc.read(&mut left_pot).unwrap();
                        let power = adc.read(&mut right_pot).unwrap();

                        let frame = Frame {
                            motor_state: MotorState::from_pot(power),
                            motor_direction: MotorDirection::from_pot(dir)
                        };
                        
                        frame.send(&mut tx_l);
                        frame.send(&mut tx_r);
                    }
                }
                */
            }
            Err(_) => (),
        }
    }
}
