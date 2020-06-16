use core::cell::RefCell;
use core::marker::PhantomData;
use embedded_hal::adc::{Channel, OneShot};
use stm32f1xx_hal::prelude::*;

pub trait Adc {
    type Sample;
    fn read(&mut self) -> Self::Sample;
}

pub struct RefAdc<'a, AdcDev, Word, Pin, Adc> {
    pin: Pin,
    adc: &'a RefCell<Adc>,
    _phantom: PhantomData<(AdcDev, Word)>,
}

impl<'a, AdcDev, Word, Pin: Channel<AdcDev>, Adc: OneShot<AdcDev, Word, Pin>>
    RefAdc<'a, AdcDev, Word, Pin, Adc>
{
    pub fn new(adc: &'a RefCell<Adc>, pin: Pin) -> Self {
        RefAdc {
            pin,
            adc,
            _phantom: PhantomData,
        }
    }
}

impl<'a, AdcDev, Word, Pin: Channel<AdcDev>, Adc_: OneShot<AdcDev, Word, Pin>> Adc
    for RefAdc<'a, AdcDev, Word, Pin, Adc_>
{
    type Sample = Word;

    fn read(&mut self) -> Self::Sample {
        match self.adc.borrow_mut().read(&mut self.pin) {
            Ok(x) => x,
            _ => panic!("Failed when reading ADC"),
        }
    }
}
