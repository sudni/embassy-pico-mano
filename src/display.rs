#![allow(dead_code)]
use core::f32::consts::PI;
use embassy_rp::gpio::Output;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::mono_font::{MonoTextStyle, ascii::FONT_10X20};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Alignment, Text};
use micromath::F32Ext;

pub const WIDTH: usize = 240;
pub const HEIGHT: usize = 320;

pub struct FrameBuffer {
    pub pixels: &'static mut [Rgb565; WIDTH * HEIGHT],
}

impl DrawTarget for FrameBuffer {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            if coord.x >= 0 && coord.x < WIDTH as i32 && coord.y >= 0 && coord.y < HEIGHT as i32 {
                let index = coord.y as usize * WIDTH + coord.x as usize;
                self.pixels[index] = color;
            }
        }
        Ok(())
    }
}

impl OriginDimensions for FrameBuffer {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

impl FrameBuffer {
    pub fn clear(&mut self, color: Rgb565) {
        self.pixels.fill(color);
    }
}

pub struct Rng(pub u32);
impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }
    pub fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
}

pub async fn animation_text<D>(display: &mut D, led: &mut Output<'_>, rng: &mut Rng) -> Duration
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

pub async fn animation_circles<D>(display: &mut D, led: &mut Output<'_>) -> Duration
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

pub async fn animation_pixels<D>(display: &mut D, led: &mut Output<'_>, rng: &mut Rng) -> Duration
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

pub async fn animation_tunnel<D>(display: &mut D, led: &mut Output<'_>) -> Duration
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

pub const CIRCLE_COLORS: [Rgb565; 12] = [
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

pub async fn show_fps<D>(display: &mut D, duration: Duration)
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
