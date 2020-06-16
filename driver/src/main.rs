#![no_std]
#![no_main]
#![allow(deprecated)]

//extern crate panic_halt;
extern crate panic_semihosting;

#[macro_use]
extern crate cortex_m_semihosting;

use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::timer::{CountDown, Periodic};

use cortex_m_rt::entry;

use stm32f1::stm32f103::interrupt;
use stm32f1xx_hal::adc::Adc;
use stm32f1xx_hal::gpio::{gpioa, gpiob, gpioc, Analog, Input, Output, PullUp, PushPull};
use stm32f1xx_hal::pac;
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::serial::{Event, Rx, Serial, Tx};
use stm32f1xx_hal::time::Hertz;
use stm32f1xx_hal::timer::{self, Timer};

use core::cell::{Cell, RefCell};
use cortex_m::interrupt::Mutex;

use common::*;

mod stepper;
use stepper::*;

mod mpsc;

#[cfg(feature = "left")]
mod consts {
    pub const ID: u8 = 1;

    pub const GEAR_REV: u16 = 1540;
    pub const GEAR_FWD: u16 = 2630;
    pub const GEAR_IDLE: u16 = (GEAR_REV + GEAR_FWD) / 2;

    pub const THROTTLE_MIN: u16 = 2720; //1500;
    pub const THROTTLE_MAX: u16 = 1500; // 2720;

    pub const STEPPER_LIM_R: i32 = 800*18;
}
#[cfg(feature = "right")]
mod consts {
    pub const ID: u8 = 2;
    pub const GEAR_REV: u16 = 1883;
    pub const GEAR_FWD: u16 = 2712;
    pub const GEAR_IDLE: u16 = (GEAR_REV + GEAR_FWD) / 2;

    pub const THROTTLE_MIN: u16 = 1900;
    pub const THROTTLE_MAX: u16 = 2790;

    pub const STEPPER_LIM_R: i32 = 800*18;
}

use consts::*;


static STEPPER_CONTROLLER: StepperController = StepperController::new();

#[rtfm::app(device = stm32f1xx_hal::pac)]
const APP: () = {
    static mut GEAR: common::Actuator<
        gpioa::PA1<Output<PushPull>>,
        gpioa::PA2<Output<PushPull>>,
        gpioa::PA0<Analog>,
        gpioa::PA3<Input<PullUp>>,
    > = ();

    static mut THROTTLE: common::Actuator<
        gpioa::PA5<Output<PushPull>>,
        gpioa::PA6<Output<PushPull>>,
        gpioa::PA4<Analog>,
        gpioa::PA7<Input<PullUp>>,
    > = ();

    static mut ADC: Adc<pac::ADC1> = ();

    static mut TIMER_HANDLE: Timer<pac::SYST> = ();

    static mut STEPPER: Stepper<'static,
        gpiob::PB3<Output<PushPull>>,
        gpiob::PB7<Output<PushPull>>,
        gpiob::PB5<Output<PushPull>>,
        gpiob::PB6<Input<PullUp>>,
        gpiob::PB4<Input<PullUp>>,
        gpiob::PB9<Input<PullUp>>,
        gpiob::PB8<Input<PullUp>>,
        Timer<pac::TIM2>,
    > = ();

    static mut MOTOR_STATE: common::MotorState = common::MotorState::Idle(0);

    static mut CLOCK: Timer<pac::TIM1> = ();

    static mut RX: Rx<pac::USART1> = ();
    static mut TX: Tx<pac::USART1> = ();

    static mut PC13: gpioc::PC13<Output<PushPull>> = ();

    #[init]
    fn init() -> init::LateResources {
        // Get access to the core peripherals from the cortex-m crate
        let cp = core;
        // Get access to the device specific peripherals from the peripheral access crate
        let dp = device;

        // Take ownership over the raw flash and rcc devices and convert them into the corresponding
        // HAL structs
        let mut flash = dp.FLASH.constrain();
        let mut rcc = dp.RCC.constrain();
        let mut afio = dp.AFIO.constrain(&mut rcc.apb2);

        let clocks = rcc.cfgr.freeze(&mut flash.acr);

        let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);
        let mut gpiob = dp.GPIOB.split(&mut rcc.apb2);
        let mut gpioc = dp.GPIOC.split(&mut rcc.apb2);

        let mut pc13 = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
        pc13.set_low();

        let mut serial = {
            let pin_tx = gpioa.pa9.into_alternate_push_pull(&mut gpioa.crh);
            let pin_rx = gpioa.pa10;
            Serial::usart1(dp.USART1, (pin_tx, pin_rx), &mut afio.mapr, 9_600.bps(), clocks, &mut rcc.apb2)
        };
        serial.listen(Event::Rxne);
        let (tx, rx) = serial.split();

        /*
        cortex_m::interrupt::free(|cs| {
            serial.listen(Event::Rxne);
            SERIAL.borrow(&cs).replace(Some(serial.split()));
        });
        */
        let mut start_relay = gpiob.pb0.into_push_pull_output(&mut gpiob.crl);
        let mut stop_relay = gpiob.pb1.into_push_pull_output(&mut gpiob.crl);

        let mut adc = Adc::adc1(dp.ADC1, &mut rcc.apb2);

        let mut gear = {
            let pos_pin = gpioa.pa0.into_analog(&mut gpioa.crl);
            let mut in1 = gpioa.pa1.into_push_pull_output(&mut gpioa.crl);
            let mut in2 = gpioa.pa2.into_push_pull_output(&mut gpioa.crl);
            let lim = gpioa.pa3.into_pull_up_input(&mut gpioa.crl);
            common::Actuator::new(in1, in2, pos_pin, lim)
        };
        //gear.goto(10000);
        //gear.go_fwd();

        let mut throttle = {
            let pos_pin = gpioa.pa4.into_analog(&mut gpioa.crl);
            let in1 = gpioa.pa5.into_push_pull_output(&mut gpioa.crl);
            let in2 = gpioa.pa6.into_push_pull_output(&mut gpioa.crl);
            let lim = gpioa.pa7.into_pull_up_input(&mut gpioa.crl);
            common::Actuator::new(in1, in2, pos_pin, lim)
        };

        //throttle.goto(10000);
        //throttle.go_fwd();


        let mut stepper = {
            let ena = gpiob.pb3.into_push_pull_output(&mut gpiob.crl);
            let dir = gpiob.pb7.into_push_pull_output(&mut gpiob.crl);
            let pul = gpiob.pb5.into_push_pull_output(&mut gpiob.crl);

            let pend = gpiob.pb6.into_pull_up_input(&mut gpiob.crl);
            let alm = gpiob.pb4.into_pull_up_input(&mut gpiob.crl);

            let lim_r = gpiob.pb8.into_pull_up_input(&mut gpiob.crh);
            let lim_l = gpiob.pb9.into_pull_up_input(&mut gpiob.crh);

            let timer = Timer::tim2(dp.TIM2, 1000.hz(), clocks, &mut rcc.apb1);

            stepper::Stepper::new(
                ena,
                dir,
                pul,
                pend,
                alm,
                lim_l,
                lim_r,
                STEPPER_LIM_R,
                timer,
                8000,
                &STEPPER_CONTROLLER,
            )
        };
        stepper.set_speed(1);

        loop {
            stepper.step_left();
        }
        

        let mut syst = Timer::syst(cp.SYST, 1000.hz(), clocks);
        //let mut syst = Timer::syst(cp.SYST, 1.hz(), clocks);
        syst.listen(timer::Event::Update);

        let mut clock = Timer::tim1(dp.TIM1, 1.hz(), clocks, &mut rcc.apb2);

        pc13.set_high();

        init::LateResources {
            GEAR: gear,
            THROTTLE: throttle,
            ADC: adc,
            TIMER_HANDLE: syst,
            STEPPER: stepper,
            CLOCK: clock,
            RX: rx,
            TX: tx,
            PC13: pc13,
        }
    }

    #[idle(resources = [STEPPER, CLOCK, GEAR, THROTTLE, PC13])]
    fn idle() -> ! {
        #[cfg(feature = "calibration")]
        {
            resources.GEAR.lock(|gear| {
                gear.stop();
                gear.go_rev();
            });

            for _ in 0..5 {
                nb::block!(resources.CLOCK.wait());
            }

            let pos = resources.GEAR.lock(|gear| {
                gear.position()
            });

            hprintln!("Gear rev: {}", pos);

            resources.GEAR.lock(|gear| {
                gear.stop();
                gear.go_fwd();
            });

            for _ in 0..5 {
                nb::block!(resources.CLOCK.wait());
            }

            let pos = resources.GEAR.lock(|gear| {
                gear.position()
            });

            hprintln!("Gear fwd: {}", pos);

            resources.THROTTLE.lock(|throttle| {
                throttle.stop();
                throttle.go_rev();
            });

            for _ in 0..5 {
                nb::block!(resources.CLOCK.wait());
            }

            let pos = resources.THROTTLE.lock(|throttle| {
                throttle.position()
            });

            hprintln!("Throttle rev: {}", pos);

            resources.THROTTLE.lock(|throttle| {
                throttle.stop();
                throttle.go_fwd();
            });

            for _ in 0..5 {
                nb::block!(resources.CLOCK.wait());
            }

            let pos = resources.THROTTLE.lock(|throttle| {
                throttle.position()
            });

            hprintln!("Throttle fwd: {}", pos);

            loop {}
        }

        #[cfg(not(feature = "calibration"))]
        {
            // Zero stepper

            // Zero gear and throttle
            resources.GEAR.lock(|gear| {
                gear.goto(GEAR_IDLE);
            });
            resources.THROTTLE.lock(|throttle| {
                throttle.goto(THROTTLE_MIN);
            });

            resources.STEPPER.zero();

            // Run stepper
            resources.STEPPER.run();
            loop {}
        }
    }

    #[exception(priority = 1, resources = [GEAR, THROTTLE, ADC, MOTOR_STATE])]
    fn SysTick() {

        resources.GEAR.tick(resources.ADC);
        resources.THROTTLE.tick(resources.ADC);


        #[cfg(not(feature = "calibration"))]
        {
            let motor_state = resources.MOTOR_STATE.lock(|x| *x);
            match motor_state {
                common::MotorState::Idle(x) => {
                    if resources.GEAR.within(GEAR_IDLE) {
                        if !resources.GEAR.stopped() {
                            resources.GEAR.goto(GEAR_IDLE);
                        }
                        let power = common::remap(x as u32, 0, 255, THROTTLE_MIN as u32, THROTTLE_MAX as u32) as u16;
                        resources.THROTTLE.goto(power);
                    }
                    else if resources.THROTTLE.within(THROTTLE_MIN) {
                        if !resources.THROTTLE.stopped() {
                            resources.THROTTLE.goto(THROTTLE_MIN);
                        }
                        resources.GEAR.goto(GEAR_IDLE);
                    }
                    else {
                        resources.THROTTLE.goto(THROTTLE_MIN);
                    }
                }
                common::MotorState::Fwd(x) => {
                    if resources.GEAR.within(GEAR_FWD) {
                        if !resources.GEAR.stopped() {
                            resources.GEAR.goto(GEAR_FWD);
                        }
                        let power = common::remap(x as u32, 0, 255, THROTTLE_MIN as u32, THROTTLE_MAX as u32) as u16;
                        resources.THROTTLE.goto(power);
                    }
                    else if resources.THROTTLE.within(THROTTLE_MIN) {
                        if !resources.THROTTLE.stopped() {
                            resources.THROTTLE.goto(THROTTLE_MIN);
                        }
                        resources.GEAR.goto(GEAR_FWD);
                    }
                    else {
                        resources.THROTTLE.goto(THROTTLE_MIN);
                    }
                }
                common::MotorState::Rev(x) => {
                    if resources.GEAR.within(GEAR_REV) {
                        if !resources.GEAR.stopped() {
                            resources.GEAR.goto(GEAR_REV);
                        }
                        let power = common::remap(x as u32, 0, 255, THROTTLE_MIN as u32, THROTTLE_MAX as u32) as u16;
                        resources.THROTTLE.goto(power);
                    }
                    else if resources.THROTTLE.within(THROTTLE_MIN) {
                        if !resources.THROTTLE.stopped() {
                            resources.THROTTLE.goto(THROTTLE_MIN);
                        }
                        resources.GEAR.goto(GEAR_REV);
                    }
                    else {
                        resources.THROTTLE.goto(THROTTLE_MIN);
                    }
                }
            }
        }
    }
    
    #[interrupt(priority = 1, resources = [TX, RX, MOTOR_STATE])]
    fn USART1() {
        static mut parser: FrameParser = FrameParser::new();

        while let Some(frame) = parser.recv(resources.RX) {
            if frame.id != ID {
                return
            }

            //hprintln!("Frame: {:?}", frame);

            let steering_pos = common::remap(frame.motor_direction as i32, 0, 255, 0, STEPPER_LIM_R);
            STEPPER_CONTROLLER.goto(steering_pos);
            resources.MOTOR_STATE.lock(|motor_state| {
                *motor_state = frame.motor_state;
            });
                                       

        }

        //static mut parser: impl communication::Parser<Result = communication::Msg> = communication::msg_parser();
    }
};

