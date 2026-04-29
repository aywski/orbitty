mod lighting;
mod palette;
mod planets;
mod renderer;

use clap::Parser;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, terminal,
};
use planets::PlanetId;
use std::f64::consts::PI;
use std::io::{self, Write};
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(name = "orbitty", about = "Spinning planets in your terminal")]
struct Args {
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u32).range(1..=240),
          help = "Frame rate (1-240)")]
    fps: u32,

    #[arg(long, value_parser = parse_zoom,
          help = "Initial zoom level (0.3-4.0, default 0.43)")]
    zoom: Option<f64>,

    #[arg(long, value_parser = parse_speed,
          help = "Initial speed multiplier: one of -32 -16 -8 -4 -2 -1 -0.5 0 0.5 1 2 4 8 16 32 (default 4)")]
    speed: Option<f64>,

    #[arg(long, value_name = "PLANET",
          help = "Planet to display: mercury, venus, earth, mars, jupiter, saturn, uranus, neptune, random, or a hex seed (e.g. 1a2b3c4d)")]
    planet: Option<String>,
}

fn parse_zoom(s: &str) -> Result<f64, String> {
    let v: f64 = s.parse().map_err(|_| format!("invalid zoom: {s}"))?;
    if v < 0.3 || v > 4.0 {
        return Err(format!("zoom must be between 0.3 and 4.0, got {v}"));
    }
    Ok(v)
}

fn parse_speed(s: &str) -> Result<f64, String> {
    let v: f64 = s.parse().map_err(|_| format!("invalid speed: {s}"))?;
    if !SPEEDS.iter().any(|&x| (x - v).abs() < 1e-9) {
        let list = SPEEDS.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" ");
        return Err(format!("speed must be one of: {list}"));
    }
    Ok(v)
}

enum PlanetArg {
    Named(PlanetId),
    Seed(u64),
}

fn parse_planet_arg(s: &str) -> Option<PlanetArg> {
    match s.to_ascii_lowercase().as_str() {
        "mercury" => Some(PlanetArg::Named(PlanetId::Mercury)),
        "venus" => Some(PlanetArg::Named(PlanetId::Venus)),
        "earth" => Some(PlanetArg::Named(PlanetId::Earth)),
        "mars" => Some(PlanetArg::Named(PlanetId::Mars)),
        "jupiter" => Some(PlanetArg::Named(PlanetId::Jupiter)),
        "saturn" => Some(PlanetArg::Named(PlanetId::Saturn)),
        "uranus" => Some(PlanetArg::Named(PlanetId::Uranus)),
        "neptune" => Some(PlanetArg::Named(PlanetId::Neptune)),
        _ => u64::from_str_radix(s, 16).ok().map(PlanetArg::Seed),
    }
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

    if let Some(s) = &args.planet {
        if s != "random" && parse_planet_arg(s).is_none() {
            eprintln!("error: unknown planet {:?}. Use mercury, venus, earth, mars, jupiter, saturn, uranus, neptune, random, or a hex seed.", s);
            std::process::exit(1);
        }
    }

    let mut stdout = io::stdout();
    terminal::enable_raw_mode().expect("failed to enable raw mode");
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide,).unwrap();

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let mut stdout = io::stdout();
        let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
        let _ = terminal::disable_raw_mode();
        default_hook(info);
    }));

    let result = run(&mut stdout, args);

    execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show).unwrap();
    terminal::disable_raw_mode().unwrap();

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

const SPEEDS: &[f64] = &[
    -32.0, -16.0, -8.0, -4.0, -2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0,
];
const DEFAULT_SPEED_IDX: usize = 11; // 4.0
const SPIN_RATE: f64 = 2.0 * PI / 30.0;

struct RandomState {
    seed: u64,
    planet: Box<dyn planets::Planet>,
    name: String,
}

struct FallingStarAnim {
    hx: f64,
    hy: f64,
    nx: f64,
    ny: f64,
    speed: f64,
    age: f64,
    lifetime: f64,
}

fn fs_rand(n: u64) -> f64 {
    let mut h = n
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    (h as f64) / (u64::MAX as f64)
}

fn gen_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let mut h = t
        .as_secs()
        .wrapping_mul(6364136223846793005)
        .wrapping_add(t.subsec_nanos() as u64);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
    h ^= h >> 33;
    h
}

fn run(stdout: &mut impl Write, args: Args) -> io::Result<()> {
    let frame_duration = Duration::from_secs_f64(1.0 / args.fps as f64);

    let initial_zoom = args.zoom.unwrap_or(0.57 / (1.15 * 1.15));
    let initial_speed_idx = args.speed.map(|v| {
        SPEEDS.iter().position(|&x| (x - v).abs() < 1e-9).unwrap_or(DEFAULT_SPEED_IDX)
    }).unwrap_or(DEFAULT_SPEED_IDX);

    let (mut current_planet, mut current_planet_box, mut random_state) = match args.planet.as_deref() {
        Some("random") => {
            let seed = gen_seed();
            let (planet, name) = planets::make_random(seed);
            (PlanetId::Earth, planets::get(PlanetId::Earth), Some(RandomState { seed, planet, name }))
        }
        Some(s) => match parse_planet_arg(s) {
            Some(PlanetArg::Named(id)) => (id, planets::get(id), None),
            Some(PlanetArg::Seed(seed)) => {
                let (planet, name) = planets::make_random(seed);
                (PlanetId::Earth, planets::get(PlanetId::Earth), Some(RandomState { seed, planet, name }))
            }
            None => unreachable!(),
        },
        None => (PlanetId::Earth, planets::get(PlanetId::Earth), None),
    };

    let mut seed_input: Option<String> = None;
    let mut speed_idx: usize = initial_speed_idx;
    let mut zoom: f64 = initial_zoom;
    let mut show_help: bool = false;
    let mut orbit_angle: f64 = 0.0;
    let mut spin_accum: f64 = 0.0;

    let mut prev_frame: Vec<renderer::Cell> = Vec::new();
    let mut bufs = renderer::RenderBufs::new();
    let mut last_frame = Instant::now();
    let mut time_accum: f64 = 0.0;
    let mut fs_counter: u64 = gen_seed();
    let mut fs_timer: f64 = 5.0 + fs_rand(fs_counter) * 20.0;
    fs_counter += 1;
    let mut active_fs: Option<FallingStarAnim> = None;

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
                    Event::Key(KeyEvent {
                        code: KeyCode::Esc, ..
                    }) => {
                        seed_input = None;
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Enter,
                        ..
                    }) => {
                        if let Some(ref s) = seed_input {
                            if let Ok(parsed) = u64::from_str_radix(s, 16) {
                                let (planet, name) = planets::make_random(parsed);
                                random_state = Some(RandomState {
                                    seed: parsed,
                                    planet,
                                    name,
                                });
                                spin_accum = 0.0;
                                bufs.clear_trail();
                            }
                        }
                        seed_input = None;
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Backspace,
                        ..
                    }) => {
                        if let Some(ref mut s) = seed_input {
                            s.pop();
                        }
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Char(c),
                        ..
                    }) => {
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
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q' | 'й'),
                    ..
                }) => {
                    if show_help {
                        show_help = false;
                    } else {
                        return Ok(());
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc, ..
                }) => {
                    if show_help {
                        show_help = false;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('h' | 'р'),
                    ..
                }) => {
                    show_help = !show_help;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('+' | '='),
                    ..
                }) => {
                    zoom = (zoom * 1.15).min(4.0);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('-' | '_'),
                    ..
                }) => {
                    zoom = (zoom / 1.15).max(0.3);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char(']' | 'ъ'),
                    ..
                }) => {
                    speed_idx = (speed_idx + 1).min(SPEEDS.len() - 1);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('[' | 'х'),
                    ..
                }) => {
                    speed_idx = speed_idx.saturating_sub(1);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('r' | 'к'),
                    ..
                }) => {
                    let seed = gen_seed();
                    let (planet, name) = planets::make_random(seed);
                    random_state = Some(RandomState { seed, planet, name });
                    spin_accum = 0.0;
                    bufs.clear_trail();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('s' | 'ы'),
                    ..
                }) => {
                    seed_input = Some(String::new());
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char(c @ '1'..='8'),
                    ..
                }) => {
                    current_planet = PlanetId::from_digit(c.to_digit(10).unwrap() as u8);
                    current_planet_box = planets::get(current_planet);
                    random_state = None;
                    spin_accum = 0.0;
                    bufs.clear_trail();
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
        time_accum += dt;

        spin_accum += SPIN_RATE * dt * SPEEDS[speed_idx];

        let (cols, rows) = terminal::size()?;
        let width = cols as usize;
        let height = rows as usize;

        orbit_angle += 0.009 * dt;

        if let Some(ref mut fs) = active_fs {
            fs.hx += fs.nx * fs.speed * dt;
            fs.hy += fs.ny * fs.speed * dt;
            fs.age += dt;
            if fs.age >= fs.lifetime { active_fs = None; }
        }
        fs_timer -= dt;
        if fs_timer <= 0.0 && active_fs.is_none() {
            let r0 = fs_rand(fs_counter);
            let r1 = fs_rand(fs_counter + 1);
            let r2 = fs_rand(fs_counter + 2);
            let r3 = fs_rand(fs_counter + 3);
            fs_counter += 4;
            let edge = (r0 * 3.0) as u8;
            let angle = (10.0 + r1 * 20.0) * PI / 180.0;
            let margin = 0.15;
            let (nx, ny, hx, hy) = match edge {
                0 => (angle.cos(), angle.sin(), 0.0,
                    (margin + r2 * (1.0 - 2.0 * margin)) * height as f64),
                1 => (-angle.cos(), angle.sin(), width as f64 - 1.0,
                    (margin + r2 * (1.0 - 2.0 * margin)) * height as f64),
                _ => {
                    let dir = if r3 < 0.5 { 1.0_f64 } else { -1.0_f64 };
                    (dir * angle.cos(), angle.sin(),
                    (margin + r2 * (1.0 - 2.0 * margin)) * width as f64, 0.0)
                }
            };
            let speed = 35.0 + fs_rand(fs_counter) * 30.0;
            fs_counter += 1;
            let lifetime = ((width as f64 * 1.3) / speed).max(1.0);
            active_fs = Some(FallingStarAnim { hx, hy, nx, ny, speed, age: 0.0, lifetime });
            fs_timer = 50.0 + fs_rand(fs_counter) * 20.0;
            fs_counter += 1;
        }

        let falling_star = active_fs.as_ref().map(|fs| {
            let progress = fs.age / fs.lifetime;
            let alpha = if progress < 0.1 {
                progress / 0.1
            } else if progress > 0.8 {
                (1.0 - progress) / 0.2
            } else {
                1.0
            };
            renderer::FallingStar { hx: fs.hx, hy: fs.hy, alpha }
        });

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
            time: time_accum,
            falling_star,
        };
        let frame = renderer::render(&scene, &mut bufs);
        renderer::flush(stdout, &frame, &prev_frame, width, height)?;
        prev_frame = frame;
    }
}
