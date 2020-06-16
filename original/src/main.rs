#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(deprecated)]
#![feature(generators, generator_trait)]
#![feature(conservative_impl_trait)]

extern crate panic_halt;

#[macro_use]
extern crate coroutine;

use coroutine::*;

use embedded_hal::adc::OneShot;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::timer::{CountDown, Periodic};

use cortex_m_rt::entry;
use stm32f1xx_hal::{adc::Adc, pac, prelude::*, time::Hertz, timer::Timer};

use core::cell::RefCell;

mod actuator;
mod adc;
mod blocking_executor;
mod steering;

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

    let pwm_pins = (
        gpiob.pb6.into_alternate_push_pull(&mut gpiob.crl),
        gpiob.pb7.into_alternate_push_pull(&mut gpiob.crl),
        gpiob.pb8.into_alternate_push_pull(&mut gpiob.crh),
        gpiob.pb9.into_alternate_push_pull(&mut gpiob.crh),
    );

    let (pwm1, pwm2, pwm3, pwm4) =
        dp.TIM4
            .pwm(pwm_pins, &mut afio.mapr, 5.khz(), clocks, &mut rcc.apb1);

    let steering_controller = steering::SteeringController::new();

    let mut steering = {
        let ena = gpioa.pa0.into_push_pull_output(&mut gpioa.crl);
        let dir = gpioa.pa1.into_push_pull_output(&mut gpioa.crl);
        let pul = gpioa.pa2.into_push_pull_output(&mut gpioa.crl);

        let pend = gpioa.pa3.into_pull_down_input(&mut gpioa.crl);
        let alm = gpioa.pa4.into_pull_down_input(&mut gpioa.crl);

        let lim_l = gpioa.pa5.into_pull_down_input(&mut gpioa.crl);
        let lim_r = gpioa.pa6.into_pull_down_input(&mut gpioa.crl);

        let timer = Timer::tim1(dp.TIM1, 1000.hz(), clocks, &mut rcc.apb2);

        steering_controller.new_steering(ena, dir, pul, pend, alm, lim_l, lim_r, timer, 8000)
    };
    steering.set_speed(30);
    let mut steering = steering.run();


    //block!(steering.step(8000));
    //block!(steering.step(-8000));
    

    let adc1 = RefCell::new(Adc::adc1(dp.ADC1, &mut rcc.apb2));

    let controller = actuator::ActuatorController::new();

    let mut actuator = {
        let pin = gpiob.pb0.into_analog(&mut gpiob.crl);
        let pos = adc::RefAdc::new(&adc1, pin);

        let in1 = pwm1;
        let in2 = pwm2;

        let lim = gpiob.pb2.into_pull_down_input(&mut gpiob.crl);

        controller.new_actuator(in1, in2, pos, lim)
    };
    //actuator.go_fwd();


    let mut timer = Timer::tim2(dp.TIM2, 10.hz(), clocks, &mut rcc.apb1);

    controller.set_target(2000);

    let mut control = || {
        loop {
            for p in &[1532, 2542, 2133, 2900] {
                for i in 1..50 {
                    await_nb!(timer.wait());
                }

                controller.set_target(*p);
                while !controller.in_position() {
                    yield
                }
            }
        }
    }; 

    loop {
        //actuator.go_fwd();
        actuator.tick();
        control.tick();
        steering.tick();
        //block!(actuator.goto(5000));
        /*
        actuator.go_rev();
        for i in 0..10 {
            nb::block!(timer.wait());
        }
        actuator.go_fwd();
        for i in 0..10 {
            nb::block!(timer.wait());
        }*/

        //block!(actuator.goto(2000));

        /*
        for p in &[12321, 53534, 32424, 21332] {
            block!(actuator.goto(*p));
        }
        */
    }
}
