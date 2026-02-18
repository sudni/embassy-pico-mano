#![no_std]
#![no_main]

use core::f32::consts::PI;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::spi::{Config, Spi};
use embassy_time::{Delay, Duration, Instant, Timer};
use embedded_graphics::mono_font::{MonoTextStyle, ascii::FONT_10X20};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Alignment, Text};
use embedded_hal_bus::spi::ExclusiveDevice;
use micromath::F32Ext;
use mipidsi::Builder;
use {defmt_rtt as _, panic_probe as _};

struct Rng(u32);
impl Rng {
    fn new(seed: u32) -> Self {
        Self(seed)
    }
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
}

const CIRCLE_COLORS: [Rgb565; 12] = [
    Rgb565::RED,
    Rgb565::GREEN,
    Rgb565::BLUE,
    Rgb565::BLACK,
    Rgb565::MAGENTA,
    Rgb565::CYAN,
    Rgb565::new(31, 31, 0),
    Rgb565::new(31, 15, 0),
    Rgb565::new(15, 31, 0),
    Rgb565::new(0, 31, 15),
    Rgb565::new(15, 0, 31),
    Rgb565::new(31, 0, 15),
];

async fn show_fps<D>(display: &mut D, duration: Duration)
where
    D: DrawTarget<Color = Rgb565>,
{
    let micros = duration.as_micros();
    if micros > 0 {
        let fps = 1_000_000 / micros;
        let mut buf = [0u8; 16];
        let fps_text = {
            let mut val = fps;
            let mut i = 0;
            if val == 0 {
                buf[0] = b'0';
                i = 1;
            } else {
                let mut temp = [0u8; 10];
                let mut j = 0;
                while val > 0 {
                    temp[j] = (val % 10) as u8 + b'0';
                    val /= 10;
                    j += 1;
                }
                while j > 0 {
                    j -= 1;
                    buf[i] = temp[j];
                    i += 1;
                }
            }
            core::str::from_utf8(&buf[..i]).unwrap_or("?")
        };

        let style = MonoTextStyle::new(&FONT_10X20, Rgb565::BLACK);
        Text::with_alignment(fps_text, Point::new(10, 20), style, Alignment::Left)
            .draw(display)
            .ok();
    }
}

async fn animation_text<D>(display: &mut D, led: &mut Output<'_>, rng: &mut Rng) -> Duration
where
    D: DrawTarget<Color = Rgb565>,
{
    let start = Instant::now();
    let bounds = display.bounding_box();
    let text = "-=Ewen=-";

    // Calculate text size dynamically
    let style_measure = MonoTextStyle::new(&FONT_10X20, Rgb565::BLACK);
    let text_bbox = Text::new(text, Point::zero(), style_measure).bounding_box();
    let text_width = text_bbox.size.width as i32;
    let text_height = text_bbox.size.height as i32;

    let mut pos = Point::new(
        (rng.next() % (bounds.size.width - text_width as u32)) as i32,
        (rng.next() % (bounds.size.height - text_height as u32)) as i32 + text_height,
    );
    let mut vel = Point::new(2, 2);
    let mut color_idx = 0;

    // Initial clear
    display.clear(Rgb565::BLACK).ok();

    for _ in 0..1000 {
        // 1. Erase previous position using a solid black rectangle
        let style_erase = MonoTextStyle::new(&FONT_10X20, Rgb565::BLACK);
        let bbox = Text::new(text, pos, style_erase).bounding_box();
        Rectangle::new(bbox.top_left, bbox.size)
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(display)
            .ok();

        // 2. Update position
        let next_pos = pos + vel;
        let mut hit = false;

        if next_pos.x <= 0 || next_pos.x + text_width >= bounds.size.width as i32 {
            vel.x = -vel.x;
            hit = true;
        }
        if next_pos.y <= 0 || next_pos.y + text_height >= bounds.size.height as i32 {
            vel.y = -vel.y;
            hit = true;
        }

        if hit {
            // Change color and ensure we don't pick Black (index 3) on a Black background
            color_idx = (color_idx + 1) % CIRCLE_COLORS.len();
            if CIRCLE_COLORS[color_idx] == Rgb565::BLACK {
                color_idx = (color_idx + 1) % CIRCLE_COLORS.len();
            }
            led.set_high();
        } else {
            led.set_low();
        }

        pos += vel;

        // 3. Draw at new position
        let style_draw = MonoTextStyle::new(&FONT_10X20, CIRCLE_COLORS[color_idx]);
        Text::new(text, pos, style_draw).draw(display).ok();

        // Very short delay for smooth movement
        Timer::after(Duration::from_millis(5)).await;
    }
    start.elapsed()
}

async fn animation_circles<D>(display: &mut D, led: &mut Output<'_>) -> Duration
where
    D: DrawTarget<Color = Rgb565>,
{
    let start = Instant::now();
    display.clear(Rgb565::WHITE).ok();
    let center = display.bounding_box().center();
    let radius = 30;

    Circle::new(center - Point::new(radius, radius), (radius * 2) as u32)
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::BLACK, 2))
        .draw(display)
        .ok();

    for i in 0..12 {
        let angle_deg = (i * 30) as f32;
        let angle_rad = angle_deg * (PI / 180.0);
        let x = center.x + (radius as f32 * angle_rad.cos()) as i32;
        let y = center.y + (radius as f32 * angle_rad.sin()) as i32;
        let satellite_center = Point::new(x, y);
        let color = CIRCLE_COLORS[i % CIRCLE_COLORS.len()];

        Circle::new(
            satellite_center - Point::new(radius, radius),
            (radius * 2) as u32,
        )
        .into_styled(PrimitiveStyle::with_stroke(color, 2))
        .draw(display)
        .ok();

        led.set_high();
        Timer::after(Duration::from_millis(50)).await;
        led.set_low();
    }
    start.elapsed()
}

async fn animation_pixels<D>(display: &mut D, led: &mut Output<'_>, rng: &mut Rng) -> Duration
where
    D: DrawTarget<Color = Rgb565>,
{
    let start = Instant::now();
    display.clear(Rgb565::WHITE).ok();
    let size = display.bounding_box().size;

    for _ in 0..(size.width * size.height) / 32 {
        let r = (rng.next() & 0x1F) as u8;
        let g = (rng.next() & 0x3F) as u8;
        let b = (rng.next() & 0x1F) as u8;
        let color = Rgb565::new(r, g, b);

        let x = ((rng.next() % (size.width / 4)) * 4) as i32;
        let y = ((rng.next() % (size.height / 4)) * 4) as i32;

        Rectangle::new(Point::new(x, y), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(display)
            .ok();

        if rng.next() % 100 == 0 {
            led.set_high();
            Timer::after(Duration::from_millis(1)).await;
            led.set_low();
        }
    }
    start.elapsed()
}

async fn animation_tunnel<D>(display: &mut D, led: &mut Output<'_>) -> Duration
where
    D: DrawTarget<Color = Rgb565>,
{
    let start = Instant::now();
    let bounds = display.bounding_box();
    let center = bounds.center();
    let num_rings = 10;
    let mut ring_pos = [0f32; 10];
    for i in 0..num_rings {
        ring_pos[i] = i as f32 * 20.0;
    }

    for _ in 0..300 {
        display.clear(Rgb565::BLACK).ok();

        for i in 0..num_rings {
            ring_pos[i] += 4.0; // Increased from 2.0 to 4.0
            if ring_pos[i] > 200.0 {
                ring_pos[i] = 0.0;
            }

            // Using power of 2 for a "depth" effect where circles speed up as they get closer
            let radius = ((ring_pos[i] * ring_pos[i]) / 120.0) as u32 + 2;
            let color = CIRCLE_COLORS[i % CIRCLE_COLORS.len()];

            if color == Rgb565::BLACK {
                continue;
            }

            Circle::new(
                center - Point::new(radius as i32, radius as i32),
                radius * 2,
            )
            .into_styled(PrimitiveStyle::with_stroke(color, 2))
            .draw(display)
            .ok();
        }

        led.toggle();
        // Removed Timer::after delay to run at max SPI speed
        Timer::after(Duration::from_micros(100)).await;
    }
    start.elapsed()
}

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
        .orientation(
            mipidsi::options::Orientation::default()
                .rotate(mipidsi::options::Rotation::Deg0)
                .flip_horizontal(),
        )
        .init(&mut delay)
        .unwrap();

    info!("Display initialized!");

    let mut led = Output::new(p.PIN_25, Level::Low);
    let mut rng = Rng::new(0xACE1);

    loop {
        info!("Animation 1: Bouncing Text");
        let dur = animation_text(&mut display, &mut led, &mut rng).await;
        show_fps(&mut display, dur).await;
        Timer::after(Duration::from_secs(1)).await;

        info!("Animation 2: Circles");
        let dur = animation_circles(&mut display, &mut led).await;
        show_fps(&mut display, dur).await;
        Timer::after(Duration::from_secs(1)).await;

        info!("Animation 3: Pixels");
        let dur = animation_pixels(&mut display, &mut led, &mut rng).await;
        show_fps(&mut display, dur).await;
        Timer::after(Duration::from_secs(1)).await;

        info!("Animation 4: Tunnel");
        let dur = animation_tunnel(&mut display, &mut led).await;
        show_fps(&mut display, dur).await;
        Timer::after(Duration::from_secs(1)).await;
    }
}

#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}
