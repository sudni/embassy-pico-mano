#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::spi::{Config, Spi};
use embassy_time::{Delay, Duration, Timer};
use embedded_graphics::mono_font::{ascii::FONT_10X20, MonoTextStyle};
use embedded_graphics::text::{Alignment, Text};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::Builder;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting Embassy on RP2040 with ILI9341 display!");

    let p = embassy_rp::init(Default::default());

    // SPI0 configuration
    let mut spi_config = Config::default();
    spi_config.frequency = 64_000_000; // 64MHz for fast display updates
    
    // SCK = GP18, MOSI = GP19, MISO = GP16 (unused)
    let spi = Spi::new_blocking(p.SPI0, p.PIN_18, p.PIN_19, p.PIN_16, spi_config);

    // GPIO configuration
    let dc = Output::new(p.PIN_20, Level::Low);
    let cs = Output::new(p.PIN_21, Level::High); // CS is active low
    let rst = Output::new(p.PIN_17, Level::Low);

    // Delay provider for initialization and SPI device
    let mut delay = Delay;

    // Create a SpiDevice from the SpiBus and CS pin
    // Use ExclusiveDevice to handle CS automatically
    let spi_device = ExclusiveDevice::new(spi, cs, Delay).unwrap();

    // mipidsi 0.9.0 needs a buffer for its SpiInterface
    let mut buffer = [0u8; 1024];
    let di = mipidsi::interface::SpiInterface::new(spi_device, dc, &mut buffer);

    // Initialize ILI9341
    let mut display = Builder::new(mipidsi::models::ILI9341Rgb565, di)
        .reset_pin(rst)
        .display_size(240, 320)
        .orientation(mipidsi::options::Orientation::default()
            .rotate(mipidsi::options::Rotation::Deg0)
            .flip_horizontal())
        .init(&mut delay)
        .unwrap();

    info!("Display initialized!");

    let mut led = Output::new(p.PIN_25, Level::Low);
    let colors = [
        Rgb565::RED,
        Rgb565::GREEN,
        Rgb565::BLUE,
        Rgb565::BLACK,
        Rgb565::WHITE,
        Rgb565::YELLOW,
        Rgb565::CYAN,
        Rgb565::MAGENTA,
    ];

    loop {
        for color in colors {
            info!("Clearing screen...");
            display.clear(color).unwrap();

            // Choose text color (white on dark, black on light)
            let text_color = if color == Rgb565::WHITE || color == Rgb565::YELLOW || color == Rgb565::CYAN {
                Rgb565::BLACK
            } else {
                Rgb565::WHITE
            };

            let style = MonoTextStyle::new(&FONT_10X20, text_color);
            
            // Draw "SuDnI" centered
            Text::with_alignment(
                "SuDnI",
                display.bounding_box().center(),
                style,
                Alignment::Center,
            )
            .draw(&mut display)
            .unwrap();
            
            led.set_high();
            Timer::after(Duration::from_millis(500)).await;
            led.set_low();
            Timer::after(Duration::from_millis(500)).await;
        }
    }
}

#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}
