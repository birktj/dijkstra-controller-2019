use embedded_hal::timer::{CountDown, Periodic};
use embedded_hal::digital::{InputPin, OutputPin};

trait Component {
    type State;

    type Params;

    type Msg;

    fn init(params: Self::Params) -> Self::State;

    fn update(&mut self, msg: Self::Msg);

    fn step(&mut self);
}


fn step(&mut self) {
    if self.reached_limit() {
        self.steering.stop();
    }
    self.steering.step();
    sequence!{
        self.steering.speed(10);
        self.steering.fwd();
    }
}

enum Event {
    Limit1,
    Limit2,
    GoTo(u32),
}

struct State {
    
}

impl InitState {
    fn update(&mut self, event: Some(Event)) {
        match event {
            None => 
        }
        if servo.ready() {
            if !limit1() {
                servo.step(1);
                length++;
            }
        }
        servo.update(None);
    }
}


struct State {
    
}


impl Servo {
    type Error = !;

    fn step(&mut self, steps: u32) {
        
    }

    fn update(&mut self) -> Result<bool, Self::Error> {
        if self.pos != self.target {}
    }

    fn wait(&mut self) -> Result<(), Self::Error> {
        while self.update()? {}
        Ok(())
    }
}



struct HybridServo<PUL, DIR, TIM> {
    pul: PUL,
    dir: DIR,
    timer: TIM,
    ppr: u32,
    rpm: u32,
    pos: i64,
    target: i64,
    high: bool,
    cw: bool,
}

impl<PUL: OutputPin, DIR: OutputPin, TIM: CountDown + Periodic> HybridServo<PUL, DIR, TIM> where TIM::Time: From<Hertz> {
    pub fn new(mut pul: PUL, dir: DIR, timer: TIM, ppr: u32) -> Self {
        HybridServo {
            pul,
            dir,
            timer,
            ppr,
            rpm: 30,
            pos: 0,
            target: 0,
            high: false,
            cw: true,
        }
    }

    fn rotate(&mut self, steps: i64) {
        self.target += steps;
        block!(self.step());
    }

    fn step(&mut self) -> nb::Result<(), Void> {
        if self.pos != self.target {
            if self.pos > self.target && !self.cw {
                self.dir.set_low();
                self.cw = true;
            }
            if self.pos < self.target && self.cw {
                self.dir.set_high();
                self.cw = false;
            }
            self.timer.wait()?;
            if self.high {
                self.pul.set_low();
                self.high = false;
                self.pos += if self.cw { -1 } else { 1 };
            }
            else {
                self.pul.set_high();
                self.high = true;
            }
        }
        Ok(())
    }
}
