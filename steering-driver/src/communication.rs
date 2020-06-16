use byteorder::{ByteOrder, LE};

#[repr(C)]
pub enum Msg {
    Zero,
    Goto(i32),
}

impl Msg {
    fn parse(frame: &[u8]) -> Option<Msg> {
        match frame[0] {
            1 => Some(Msg::Zero),
            2 => Some(Msg::Goto(LE::read_i32(&frame[1..5]))),
            _ => None
        }
    }

    fn write(&self, frame: &mut [u8]) {
        match self {
            Msg::Zero => {
                frame[0] = 1;
            }
            Msg::Goto(a) => {
                frame[0] = 1;
                LE::write_i32(&mut frame[1..5], a);
            }
        }
    }
}

pub struct SliceReader<'a>{
    data: &'a mut [u8],
    i: usize,
}

impl<'a> SliceReader<'a> {
    pub fn feed(&mut self, byte: u8) -> bool {
        self.data[self.i] = byte;
        self.i = (self.i + 1) % self.data.len();
        self.i == 0
    }
}
