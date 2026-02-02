#![no_std]
#![no_main]


use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting Embassy on RP2040!");
    
    let p = embassy_rp::init(Default::default());
    let mut led = gpio::Output::new(p.PIN_25, gpio::Level::Low);
    
    info!("LED blinking started!");
    
    loop {
        led.set_high();
        Timer::after(Duration::from_millis(500)).await;
        led.set_low();
        Timer::after(Duration::from_millis(500)).await;
    }
}

#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}
