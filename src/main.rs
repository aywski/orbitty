mod renderer;
mod lighting;
mod palette;
mod planets;

use std::f64::consts::PI;
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

const SPEEDS: &[f64] = &[-32.0, -16.0, -8.0, -4.0, -2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0];
const DEFAULT_SPEED_IDX: usize = 11; // 1.0
const SPIN_RATE: f64 = 2.0 * PI / 30.0;

struct RandomState {
    seed: u64,
    planet: Box<dyn planets::Planet>,
    name: String,
}

fn gen_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let mut h = t.as_secs().wrapping_mul(6364136223846793005)
        .wrapping_add(t.subsec_nanos() as u64);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
    h ^= h >> 33;
    h
}

fn run(stdout: &mut impl Write, fps: u32) -> io::Result<()> {
    let frame_duration = Duration::from_secs_f64(1.0 / fps as f64);

    let mut current_planet = PlanetId::Earth;
    let mut current_planet_box: Box<dyn planets::Planet> = planets::get(current_planet);
    let mut random_state: Option<RandomState> = None;
    let mut seed_input: Option<String> = None;
    let mut speed_idx: usize = DEFAULT_SPEED_IDX;
    let mut zoom: f64 = 0.57;
    let mut show_help: bool = false;
    let mut orbit_angle: f64 = 0.0;
    let mut spin_accum: f64 = 0.0;

    let mut prev_frame: Vec<renderer::Cell> = Vec::new();
    let mut trail: renderer::TrailBuf = Vec::new();
    let mut last_frame = Instant::now();

    loop {
        while event::poll(Duration::ZERO)? {
            // Seed input mode: only handle input-related keys
            if seed_input.is_some() {
                match event::read()? {
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    }) => return Ok(()),
                    Event::Key(KeyEvent { code: KeyCode::Esc, .. }) => {
                        seed_input = None;
                    }
                    Event::Key(KeyEvent { code: KeyCode::Enter, .. }) => {
                        if let Some(ref s) = seed_input {
                            if let Ok(parsed) = u64::from_str_radix(s, 16) {
                                let (planet, name) = planets::make_random(parsed);
                                random_state = Some(RandomState { seed: parsed, planet, name });
                                spin_accum = 0.0;
                                trail.clear();
                            }
                        }
                        seed_input = None;
                    }
                    Event::Key(KeyEvent { code: KeyCode::Backspace, .. }) => {
                        if let Some(ref mut s) = seed_input {
                            s.pop();
                        }
                    }
                    Event::Key(KeyEvent { code: KeyCode::Char(c), .. }) => {
                        if c.is_ascii_hexdigit() {
                            if let Some(ref mut s) = seed_input {
                                if s.len() < 16 {
                                    s.push(c.to_ascii_lowercase());
                                }
                            }
                        }
                    }
                    _ => {}
                }
                continue;
            }

            match event::read()? {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }) => return Ok(()),
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
                    let seed = gen_seed();
                    let (planet, name) = planets::make_random(seed);
                    random_state = Some(RandomState { seed, planet, name });
                    spin_accum = 0.0;
                    trail.clear();
                }
                Event::Key(KeyEvent { code: KeyCode::Char('s' | 'ы'), .. }) => {
                    seed_input = Some(String::new());
                }
                Event::Key(KeyEvent { code: KeyCode::Char(c @ '1'..='8'), .. }) => {
                    current_planet = PlanetId::from_digit(c.to_digit(10).unwrap() as u8);
                    current_planet_box = planets::get(current_planet);
                    random_state = None;
                    spin_accum = 0.0;
                    trail.clear();
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

        spin_accum += SPIN_RATE * dt * SPEEDS[speed_idx];

        let (cols, rows) = terminal::size()?;
        let width = cols as usize;
        let height = rows as usize;

        orbit_angle += 0.0003;

        let (planet_dyn, planet_name, planet_seed): (&dyn planets::Planet, &str, Option<u64>) =
            if let Some(ref rs) = random_state {
                (rs.planet.as_ref(), rs.name.as_str(), Some(rs.seed))
            } else {
                (current_planet_box.as_ref(), current_planet.name(), None)
            };

        let scene = renderer::Scene {
            width,
            height,
            planet: planet_dyn,
            spin: spin_accum,
            orbit_angle,
            zoom,
            planet_name,
            show_help,
            planet_seed,
            seed_input: seed_input.as_deref(),
        };
        let frame = renderer::render(&scene, &mut trail);
        renderer::flush(stdout, &frame, &prev_frame, width, height)?;
        prev_frame = frame;
    }
}
