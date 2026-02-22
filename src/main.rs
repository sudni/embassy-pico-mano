#![no_std]
#![no_main]

mod display;
mod touch;

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::i2c::{Config as I2cConfig, I2c};
use embassy_rp::spi::{Config as SpiConfig, Spi};
use embassy_time::{Delay, Duration, Instant, Timer};
use embedded_graphics::mono_font::{MonoTextStyle, ascii::FONT_10X20};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Circle;
use embedded_graphics::primitives::PrimitiveStyle;
use embedded_graphics::text::Text;
use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::Builder;
use {defmt_rtt as _, panic_probe as _};

use crate::display::{CIRCLE_COLORS, FrameBuffer, HEIGHT, Rng, WIDTH, show_fps};
use crate::touch::TouchController;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting Embassy on RP2040 with ILI9341 display!");

    let p = embassy_rp::init(Default::default());

    // SPI0 configuration
    let mut spi_config = SpiConfig::default();
    spi_config.frequency = 64_000_000;

    let spi = Spi::new(
        p.SPI0, p.PIN_18, p.PIN_19, p.PIN_16, p.DMA_CH0, p.DMA_CH1, spi_config,
    );
    let dc = Output::new(p.PIN_20, Level::Low);
    let cs = Output::new(p.PIN_21, Level::High);
    let rst = Output::new(p.PIN_17, Level::Low);

    let mut delay = Delay;
    let spi_device = ExclusiveDevice::new(spi, cs, Delay).unwrap();

    let mut buffer = [0u8; 1024];
    let di = mipidsi::interface::SpiInterface::new(spi_device, dc, &mut buffer);

    let mut display = Builder::new(mipidsi::models::ILI9341Rgb565, di)
        .reset_pin(rst)
        .display_size(240, 320)
        .orientation(
            mipidsi::options::Orientation::default()
                .rotate(mipidsi::options::Rotation::Deg0)
                .flip_horizontal(),
        )
        .init(&mut delay)
        .unwrap();

    info!("Display initialized!");

    // I2C1 configuration for touch
    let mut i2c_config = I2cConfig::default();
    i2c_config.frequency = 400_000;
    let mut i2c = I2c::new_blocking(p.I2C1, p.PIN_15, p.PIN_14, i2c_config);

    // Set FT6206 threshold to be more sensitive (Register 0x80, lower = more sensitive, default ~128 or 22)
    // We try 20 here.
    let _ = i2c.blocking_write(0x38u16, &[0x80, 20]);

    let touch_irq = Input::new(p.PIN_13, Pull::Up);
    let mut touch_ctrl = TouchController::new(i2c, touch_irq);

    let mut led = Output::new(p.PIN_25, Level::Low);
    let mut _rng = Rng::new(0xACE1);

    static mut FB_PIXELS: [Rgb565; WIDTH * HEIGHT] = [Rgb565::BLACK; WIDTH * HEIGHT];
    let fb_pixels = unsafe { &mut *core::ptr::addr_of_mut!(FB_PIXELS) };
    let mut fb = FrameBuffer { pixels: fb_pixels };

    loop {
        info!("Animation 4: Tunnel (DMA)");
        let start = Instant::now();
        for _ in 0..200 {
            fb.clear(Rgb565::BLACK);

            let center = Point::new(WIDTH as i32 / 2, HEIGHT as i32 / 2);
            for i in 0..10 {
                let r_base =
                    (i as f32 * 20.0 + (start.elapsed().as_millis() as f32 / 10.0)) % 200.0;
                let radius = ((r_base * r_base) / 120.0) as u32 + 2;
                Circle::new(
                    center - Point::new(radius as i32, radius as i32),
                    radius * 2,
                )
                .into_styled(PrimitiveStyle::with_stroke(
                    CIRCLE_COLORS[i % CIRCLE_COLORS.len()],
                    2,
                ))
                .draw(&mut fb)
                .ok();
            }

            // Poll touch every frame for better responsiveness
            if let Ok(Some(touch_event)) = touch_ctrl.touch.get_touch_event() {
                let point = touch_event.primary_point;
                let mut buf = [0u8; 32];
                let mut pos = 0;
                for &b in b"X:" {
                    buf[pos] = b;
                    pos += 1;
                }

                let mut val = point.x;
                if val == 0 {
                    buf[pos] = b'0';
                    pos += 1;
                } else {
                    let mut temp = [0u8; 5];
                    let mut j = 0;
                    while val > 0 {
                        temp[j] = (val % 10) as u8 + b'0';
                        val /= 10;
                        j += 1;
                    }
                    while j > 0 {
                        j -= 1;
                        buf[pos] = temp[j];
                        pos += 1;
                    }
                }

                for &b in b" Y:" {
                    buf[pos] = b;
                    pos += 1;
                }
                let mut val = point.y;
                if val == 0 {
                    buf[pos] = b'0';
                    pos += 1;
                } else {
                    let mut temp = [0u8; 5];
                    let mut j = 0;
                    while val > 0 {
                        temp[j] = (val % 10) as u8 + b'0';
                        val /= 10;
                        j += 1;
                    }
                    while j > 0 {
                        j -= 1;
                        buf[pos] = temp[j];
                        pos += 1;
                    }
                }

                if let Ok(text) = core::str::from_utf8(&buf[..pos]) {
                    let style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
                    Text::new(text, Point::new(10, 20), style)
                        .draw(&mut fb)
                        .ok();
                }
            }

            display
                .set_pixels(
                    0,
                    0,
                    (WIDTH - 1) as u16,
                    (HEIGHT - 1) as u16,
                    fb.pixels.iter().cloned(),
                )
                .ok();

            led.toggle();
        }
        let dur = start.elapsed();
        show_fps(&mut display, dur).await;
        Timer::after(Duration::from_secs(1)).await;
    }
}

#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}
