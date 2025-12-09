use std::io::{stdout, Write};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::{
    cursor,
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

fn draw_frame<W: Write>(out: &mut W, t: f32, width: u16, height: u16) -> crossterm::Result<()> {
    // simple animated ASCII heightfield / waves
    for y in 0..height {
        execute!(out, cursor::MoveTo(0, y))?;
        let ny = (y as f32 / height as f32 - 0.5) * 2.0;
        for x in 0..width {
            let nx = (x as f32 / width as f32 - 0.5) * 2.0;
            let v = ((nx * 4.0 + t).sin() + (ny * 4.0 - t * 0.7).cos()) * 0.5;
            let shade = ((v + 1.0) * 0.5).clamp(0.0, 1.0);
            let idx = (shade * (ASCII_LUT.len() - 1) as f32) as usize;
            let ch = ASCII_LUT[idx];
            execute!(out, Print(ch))?;
        }
    }
    Ok(())
}

static ASCII_LUT: &[char] = &[' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];

fn main() -> crossterm::Result<()> {
    let mut stdout = stdout();

    execute!(stdout, EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    let res = (|| -> crossterm::Result<()> {
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


