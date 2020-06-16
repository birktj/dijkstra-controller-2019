#![no_std]
#![no_main]
#![allow(deprecated)]

extern crate panic_halt;

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

use cortex_m::interrupt::{Mutex};
use core::cell::{Cell, RefCell};

use common::*;

mod stepper;
use stepper::*;

mod mpsc;
mod communication;

static STEPPER_CONTROLLER: StepperController = StepperController::new();
static SERIAL: Mutex<Cell<Option<(Tx<USART1>, Rx<USART1>)>>> = Mutex::new(Cell::new(None));

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

    let mut serial = {
        let pin_tx = gpioa.pa9.into_alternate_push_pull(&mut gpioa.crh);
        let pin_rx = gpioa.pa10;
        Serial::usart1(dp.USART1, (pin_tx, pin_rx), &mut afio.mapr, 9_600.bps(), clocks, &mut rcc.apb2)
    };

    
    cortex_m::interrupt::free(|cs| {
        serial.listen(Event::Rxne);
        SERIAL.borrow(&cs).replace(Some(serial.split()));
    });

    let mut stepper = {
        let ena = gpioa.pa0.into_push_pull_output(&mut gpioa.crl);
        let dir = gpioa.pa1.into_push_pull_output(&mut gpioa.crl);
        let pul = gpioa.pa2.into_push_pull_output(&mut gpioa.crl);

        let pend = gpioa.pa3.into_pull_down_input(&mut gpioa.crl);
        let alm = gpioa.pa4.into_pull_down_input(&mut gpioa.crl);

        let lim_l = gpioa.pa5.into_pull_down_input(&mut gpioa.crl);
        let lim_r = gpioa.pa6.into_pull_down_input(&mut gpioa.crl);

        let timer = Timer::tim1(dp.TIM1, 1000.hz(), clocks, &mut rcc.apb2);

        stepper::Stepper::new(ena, dir, pul, pend, alm, lim_l, lim_r, timer, 8000, &STEPPER_CONTROLLER)
    };
    stepper.set_speed(30);

    stepper.run()
}

#[interrupt]
fn USART1() {
    static mut tx: Option<Tx<USART1>> = None;
    static mut rx: Option<Rx<USART1>> = None;

    if tx.is_none() && rx.is_none() {
        cortex_m::interrupt::free(|cs| {
            let (t, r) = SERIAL.borrow(&cs).replace(None).unwrap();
            *tx = Some(t);
            *rx = Some(r);
        });
    }

    static mut parser = FrameParser::new();

    if let Some(frame) = parser.recv(rx.as_mut().unwrap()) {
    }

    //static mut parser: impl communication::Parser<Result = communication::Msg> = communication::msg_parser();
}
