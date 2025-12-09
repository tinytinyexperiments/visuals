use std::io::{stdout, Write, Result};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::{
    cursor,
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

fn draw_frame<W: Write>(out: &mut W, t: f32, width: u16, height: u16) -> Result<()> {
    // animated ASCII Mandelbrot zoom with color cycling
    let (w, h) = (width as f32, height as f32);
    // terminal cells are usually taller than they are wide, so compensate a bit
    let aspect = if h > 0.0 { (w / h) * 0.5 } else { 1.0 };

    let zoom = 1.0 + 0.5 * (t * 0.2).sin();
    let cx = -0.5 + 0.3 * (t * 0.05).cos();
    let cy = 0.0 + 0.3 * (t * 0.05).sin();

    let max_iter: i32 = 64;

    for y in 0..height {
        execute!(out, cursor::MoveTo(0, y))?;
        let imag = ((y as f32 / h) - 0.5) * 2.0 / zoom + cy;

        for x in 0..width {
            let real = (((x as f32 / w) - 0.5) * 3.5 * aspect) / zoom + cx;

            let mut zr = 0.0f32;
            let mut zi = 0.0f32;
            let mut iter = 0;

            while zr * zr + zi * zi <= 4.0 && iter < max_iter {
                let new_zr = zr * zr - zi * zi + real;
                let new_zi = 2.0 * zr * zi + imag;
                zr = new_zr;
                zi = new_zi;
                iter += 1;
            }

            let shade = if iter == max_iter {
                0.0
            } else {
                iter as f32 / max_iter as f32
            };

            let idx = (shade * (ASCII_LUT.len() - 1) as f32) as usize;
            let ch = ASCII_LUT[idx];

            // color based on iteration and position
            let hue = shade + (x as f32 / w) * 0.3 + (y as f32 / h) * 0.2 + t * 0.1;
            let r = (0.5 + 0.5 * (hue * 6.2831).sin()) * 255.0;
            let g = (0.5 + 0.5 * (hue * 6.2831 + 2.094).sin()) * 255.0;
            let b = (0.5 + 0.5 * (hue * 6.2831 + 4.188).sin()) * 255.0;
            execute!(
                out,
                SetForegroundColor(Color::Rgb {
                    r: r as u8,
                    g: g as u8,
                    b: b as u8
                }),
                Print(ch)
            )?;
        }
    }

    Ok(())
}

static ASCII_LUT: &[char] = &[' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];

fn main() -> Result<()> {
    let mut stdout = stdout();

    execute!(stdout, EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    let res = (|| -> Result<()> {
        let mut t: f32 = 0.0;
        loop {
            let (width, height) = terminal::size()?;
            execute!(stdout, Clear(ClearType::All), cursor::Hide)?;
            execute!(stdout, SetForegroundColor(Color::Cyan))?;

            let start = Instant::now();
            draw_frame(&mut stdout, t, width, height)?;
            execute!(stdout, ResetColor)?;
            stdout.flush().ok();

            t += 0.1;

            // ~60 FPS cap
            let frame_time = start.elapsed();
            if frame_time < Duration::from_millis(16) {
                thread::sleep(Duration::from_millis(16) - frame_time);
            }

            // simple escape: check for 'q' key without blocking
            if crossterm::event::poll(Duration::from_millis(1))? {
                if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                    if key.code == crossterm::event::KeyCode::Char('q') {
                        break;
                    }
                }
            }
        }
        Ok(())
    })();

    terminal::disable_raw_mode().ok();
    execute!(stdout, LeaveAlternateScreen, cursor::Show).ok();

    res
}


