use embassy_rp::gpio::Input;
use embassy_rp::i2c::{Blocking, I2c};
use embassy_rp::peripherals::I2C1;
use ft6x06_rs::FT6x06;

pub struct TouchController<'a> {
    pub touch: FT6x06<I2c<'a, I2C1, Blocking>>,
    #[allow(dead_code)]
    pub irq: Input<'a>,
}

impl<'a> TouchController<'a> {
    pub fn new(i2c: I2c<'a, I2C1, Blocking>, irq: Input<'a>) -> Self {
        Self {
            touch: FT6x06::new(i2c),
            irq,
        }
    }

    #[allow(dead_code)]
    pub fn is_touched(&self) -> bool {
        self.irq.is_low()
    }
}
