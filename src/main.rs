mod orbital;
mod renderer;
mod lighting;
mod palette;
mod planets;

use std::io::{self, Write};
use std::time::{Duration, Instant};
use clap::Parser;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal,
};
use planets::PlanetId;

#[derive(Parser)]
#[command(name = "orbitty", about = "Spinning planets in your terminal")]
struct Args {
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u32).range(1..=240))]
    fps: u32,
}

fn check_truecolor() {
    let colorterm = std::env::var("COLORTERM").unwrap_or_default();
    if colorterm != "truecolor" && colorterm != "24bit" {
        eprintln!("error: terminal does not support 24-bit true color");
        eprintln!("COLORTERM={:?}", colorterm);
        std::process::exit(1);
    }
}

fn main() {
    check_truecolor();
    let args = Args::parse();

    let mut stdout = io::stdout();
    terminal::enable_raw_mode().expect("failed to enable raw mode");
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        cursor::Hide,
    ).unwrap();

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let mut stdout = io::stdout();
        let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
        let _ = terminal::disable_raw_mode();
        default_hook(info);
    }));

    let result = run(&mut stdout, args.fps);

    execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show).unwrap();
    terminal::disable_raw_mode().unwrap();

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

// Discrete speed levels: negative = reverse, 0 = stopped, positive = forward.
const SPEEDS: &[f64] = &[-32.0, -16.0, -8.0, -4.0, -2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0];
const DEFAULT_SPEED_IDX: usize = 11; // 1.0

fn run(stdout: &mut impl Write, fps: u32) -> io::Result<()> {
    let frame_duration = Duration::from_secs_f64(1.0 / fps as f64);

    let mut current_planet = PlanetId::Earth;
    let mut speed_idx: usize = DEFAULT_SPEED_IDX;
    let mut zoom: f64 = 0.57;
    let mut show_help: bool = false;
    let mut orbit_angle: f64 = 0.0;
    // Accumulated spin angle - updated incrementally so speed changes don't jump.
    let mut spin_accum: f64 = 0.0;

    let mut prev_frame: Vec<renderer::Cell> = Vec::new();
    let mut last_frame = Instant::now();

    loop {
        while event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }) => return Ok(()),
                // 'q'/'й', 'h'/'р', etc. - Latin and Russian layout equivalents.
                Event::Key(KeyEvent { code: KeyCode::Char('q' | 'й'), .. }) => {
                    if show_help { show_help = false; } else { return Ok(()); }
                }
                Event::Key(KeyEvent { code: KeyCode::Esc, .. }) => {
                    if show_help { show_help = false; }
                }
                Event::Key(KeyEvent { code: KeyCode::Char('h' | 'р'), .. }) => {
                    show_help = !show_help;
                }
                Event::Key(KeyEvent { code: KeyCode::Char('+' | '='), .. }) => {
                    zoom = (zoom * 1.15).min(4.0);
                }
                Event::Key(KeyEvent { code: KeyCode::Char('-' | '_'), .. }) => {
                    zoom = (zoom / 1.15).max(0.3);
                }
                Event::Key(KeyEvent { code: KeyCode::Char(']' | 'ъ'), .. }) => {
                    speed_idx = (speed_idx + 1).min(SPEEDS.len() - 1);
                }
                Event::Key(KeyEvent { code: KeyCode::Char('[' | 'х'), .. }) => {
                    speed_idx = speed_idx.saturating_sub(1);
                }
                Event::Key(KeyEvent { code: KeyCode::Char('r' | 'к'), .. }) => {
                    spin_accum = 0.0;
                }
                Event::Key(KeyEvent { code: KeyCode::Char(c @ '1'..='8'), .. }) => {
                    current_planet = PlanetId::from_digit(c.to_digit(10).unwrap() as u8);
                    spin_accum = 0.0;
                }
                _ => {}
            }
        }

        let now = Instant::now();
        let elapsed = now.duration_since(last_frame);
        if elapsed < frame_duration {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }
        let dt = elapsed.as_secs_f64();
        last_frame = now;

        spin_accum += orbital::spin_rate() * dt * SPEEDS[speed_idx];

        let planet = planets::get(current_planet);

        let (cols, rows) = terminal::size()?;
        let width = cols as usize;
        let height = rows as usize;

        orbit_angle += 0.0003;

        let scene = renderer::Scene {
            width,
            height,
            planet: &*planet,
            spin: spin_accum,
            orbit_angle,
            zoom,
            planet_name: current_planet.name(),
            show_help,
        };
        let frame = renderer::render(&scene);
        renderer::flush(stdout, &frame, &prev_frame, width, height)?;
        prev_frame = frame;
    }
}
