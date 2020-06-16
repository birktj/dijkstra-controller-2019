enum EngineController {
    Init,
    Idle,
    Fwd,
    Rev,
}


fn main() {
    let mut throttle = ServoDriver::new();
    let mut gear     = ServoDriver::new();
    let mut steering = Stepper::new();
    

    loop {
        throttle.step();
        gear.step();
        steering.step();
    }
}

struct Controls {
    power: Relay,
    start: Relay,
    gear: Servo,
    throttle: Servo,
}

fn init(&mut self) {
    gear.goto(self.neutral_pos);
    throttle.goto(self.start_trottle_pos);
}

struct EngineController {
    state: State,
    config: Config,
    power: Relay,
    start: Relay,
    gear: Actuator,
    throttle: Actuator,
    steering: Steering,
}

fn init() -> EngineController {
    let mut dp = ...;

    let mut power = {
        let control = dp.GPIOB.pb1.into_push_pull_output();

        Relay::new(control)
    };

    let mut start = {
        let control = dp.GPIOB.pb1.into_push_pull_output();

        Relay::new(control)
    };


    let mut gear = {
        let 
    }

    Config::init();

    let mut steering = {
        let ena   = dp.GPIOA.pa1.into_push_pull_output();
        let dir   = dp.GPIOA.pa2.into_push_pull_output();
        let pul   = dp.GPIOA.pa3.into_push_pull_output();

        let pend  = dp.GPIOA.pa4.into_pulldown_input();
        let alm   = dp.GPIOA.pa5.into_pulldown_input();

        let lim_l = dp.GPIOA.pa6.into_pulldown_input();
        let lim_r = dp.GPIOA.pa7.into_pulldown_input();

        let timer = Timer::tim1(&mut dp, 10.hz())

        Steering::new(ena, dir, pul, pend, alm, lim_l, lim_r, timer)
    };

    steering.init();

    while !steering.ready() && !controls.ready() {
        steering.update();
        controls.update();
    }

    EngineController {
        state: Idle,
        steering,

    }
}
