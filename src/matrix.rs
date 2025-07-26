use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{poll, read, Event, KeyCode, KeyModifiers},
    execute, queue,
    style::{Color, Print, SetForegroundColor},
    terminal::{
        disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use ctrlc;
use rand::{thread_rng, Rng};
use rand::prelude::SliceRandom;
use std::{
    cell::RefCell,
    collections::HashMap,
    io::{stdout, Write},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    thread::sleep,
    time::{Duration, Instant},
};

thread_local! {
    pub static STUCK_HANDLER: RefCell<Box<dyn Fn(u16, u16, char)>> = RefCell::new(Box::new(|_, _, _| {}));
}

// ==== Character Sets ====
pub const MATRIX_CHARS_KATAKANA: &[char] = &[
    'ｱ','ｲ','ｳ','ｴ','ｵ','ｶ','ｷ','ｸ','ｹ','ｺ','ｻ','ｼ','ｽ','ｾ','ｿ','ﾀ','ﾁ','ﾂ','ﾃ','ﾄ',
    'ﾅ','ﾆ','ﾇ','ﾈ','ﾉ','ﾊ','ﾋ','ﾌ','ﾍ','ﾎ','ﾏ','ﾐ','ﾑ','ﾒ','ﾓ','ﾔ','ﾕ','ﾖ','ﾗ','ﾘ',
    'ﾙ','ﾚ','ﾛ','ﾜ','ﾝ',
];
pub const MATRIX_CHARS_ALPHANUMERIC: &[char] = &[
    '0','1','2','3','4','5','6','7','8','9',
    'A','B','C','D','E','F','G','H','I','J','K','L','M',
    'N','O','P','Q','R','S','T','U','V','W','X','Y','Z',
];
pub const MATRIX_CHARS_SYMBOLS: &[char] = &[
    ':','·','¦','‡','†','°','¤','═','║','╔','╗','╚','╝','╠','╣','╦','╩','╬',
];
pub const MATRIX_CHARS_GREEK: &[char] = &[
    'Α','Β','Γ','Δ','Ε','Ζ','Η','Θ','Ι','Κ','Λ','Μ','Ν','Ξ','Ο','Π','Ρ','Σ','Τ','Υ','Φ','Χ','Ψ','Ω',
];
pub const GLITCH_CHARS: &[char] = &['▒', '▓', '░', '█'];

// ==== Colors ====
const MATRIX_BRIGHT: Color = Color::White;
const MATRIX_GREEN_MID: Color = Color::Green;
const MATRIX_GREEN_DIM: Color = Color::DarkGreen;
const MATRIX_GREEN_DARK: Color = Color::DarkGreen;
const MATRIX_GREEN_DARKEST: Color = Color::Black;

// ==== Config ====
static MIN_TRAIL_ATOMIC: AtomicU32 = AtomicU32::new(8);
static MAX_TRAIL_ATOMIC: AtomicU32 = AtomicU32::new(25);
static FRAMERATE: AtomicU32 = AtomicU32::new((12.0f32).to_bits());
static STUCK_MAX_ATOMIC: AtomicU32 = AtomicU32::new(0); // 0 = unlimited
static STUCK_PROBABILITY_ATOMIC: AtomicU32 = AtomicU32::new((0.02f32).to_bits());
static GLITCH_PROBABILITY_ATOMIC: AtomicU32 = AtomicU32::new((0.003f32).to_bits());
static FLICKER_PROBABILITY_ATOMIC: AtomicU32 = AtomicU32::new((0.01f32).to_bits());
static NEW_DROP_PROBABILITY_ATOMIC: AtomicU32 = AtomicU32::new((0.05f32).to_bits());

const CHAR_CHANGE_PROBABILITY: f32 = 0.2;
const TRAIL_MIN_LIMIT: usize = 4;
const TRAIL_MAX_LIMIT: usize = 40;
const BASE_FRAME_DELAY: Duration = Duration::from_millis(60);
const SPEED_VARIATION: f32 = 0.3;
const SPEED_JITTER_PROBABILITY: f64 = 0.02;
const SPEED_JITTER_AMOUNT: f32 = 0.05;

#[derive(Clone)]
pub struct MatrixDrop<'a> {
    x: u16,
    y: f32,
    length: usize,
    speed: f32,
    chars: Vec<char>,
    last_update: Instant,
    char_change_timers: Vec<Instant>,
    charset: &'a [char],
}

impl<'a> MatrixDrop<'a> {
    pub fn new(x: u16, _rows: u16, charset: &'a [char]) -> Self {
        let mut rng = thread_rng();
        let length = rng.r#gen_range(get_min_trail()..=get_max_trail());
        let speed = 1.0 + rng.r#gen::<f32>() * SPEED_VARIATION;

        let chars = (0..length).map(|_| *charset.choose(&mut rng).unwrap()).collect();
        let char_change_timers = (0..length).map(|_| Instant::now()).collect();

        Self {
            x,
            y: -(length as f32),
            length,
            speed,
            chars,
            last_update: Instant::now(),
            char_change_timers,
            charset,
        }
    }

    pub fn update(&mut self, rows: u16) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32();
        let fps = get_framerate();
        self.y += self.speed * dt * fps;
        self.last_update = now;

        let mut rng = thread_rng();
        if rng.r#gen_bool(SPEED_JITTER_PROBABILITY) {
            let delta = rng.r#gen_range(-SPEED_JITTER_AMOUNT..SPEED_JITTER_AMOUNT);
            self.speed = (self.speed + delta).clamp(0.5, 3.0);
        }

        if self.y > rows as f32 + self.length as f32 {
            if rng.r#gen::<f32>() < get_stuck_probability() {
                let stuck_y = (self.y - self.length as f32).floor() as u16;
                if stuck_y < rows {
                    STUCK_HANDLER.with(|handler| {
                        (handler.borrow())(self.x, stuck_y, *self.chars.last().unwrap_or(&' '));
                    });
                }
            }
            *self = MatrixDrop::new(self.x, rows, self.charset);
        }

        for i in 0..self.chars.len() {
            if rng.r#gen::<f32>() < CHAR_CHANGE_PROBABILITY {
                self.chars[i] = if rng.r#gen_bool(0.005) {
                    *GLITCH_CHARS.choose(&mut rng).unwrap()
                } else {
                    *self.charset.choose(&mut rng).unwrap()
                };
                self.char_change_timers[i] = now;
            }
        }
    }

    pub fn render(
        &self,
        w: &mut impl Write,
        rows: u16,
        use_rgb_fade: bool,
        stuck_chars: &mut HashMap<u16, (u16, char)>,
    ) -> std::io::Result<()> {
        fn fade_color((r, g, b): (u8, u8, u8), alpha: f32) -> Color {
            Color::Rgb {
                r: (r as f32 * alpha).clamp(0.0, 255.0) as u8,
                g: (g as f32 * alpha).clamp(0.0, 255.0) as u8,
                b: (b as f32 * alpha).clamp(0.0, 255.0) as u8,
            }
        }

        if let Some(&(stuck_y, _)) = stuck_chars.get(&self.x) {
            if self.y >= stuck_y as f32 && self.y - self.length as f32 <= stuck_y as f32 {
                stuck_chars.remove(&self.x);
            }
        }

        for (i, &ch) in self.chars.iter().enumerate() {
            let char_y = self.y - i as f32;
            if char_y < 0.0 || char_y >= rows as f32 {
                continue;
            }

            let flicker = thread_rng().r#gen_bool(get_flicker_probability() as f64);
            let glitch = thread_rng().r#gen_bool(get_glitch_probability() as f64);

            let color = if use_rgb_fade {
                if i == 0 {
                    MATRIX_BRIGHT
                } else {
                    let alpha = 1.0 - (i as f32 / self.length as f32).powf(1.3);
                    fade_color((0, 255, 0), alpha)
                }
            } else {
                match i {
                    0 => MATRIX_BRIGHT,
                    1..=3 => MATRIX_GREEN_MID,
                    4..=8 => MATRIX_GREEN_DIM,
                    9..=15 => MATRIX_GREEN_DARK,
                    _ => MATRIX_GREEN_DARKEST,
                }
            };

            let ch = if glitch {
                *GLITCH_CHARS.choose(&mut thread_rng()).unwrap_or(&ch)
            } else if flicker {
                ' '
            } else {
                ch
            };

            queue!(w, MoveTo(self.x, char_y as u16), SetForegroundColor(color), Print(ch))?;
        }

        let tail_y = self.y - self.length as f32;
        if tail_y >= 0.0 && tail_y < rows as f32 {
            queue!(w, MoveTo(self.x, tail_y as u16), Print(' '))?;
        }

        Ok(())
    }
}

fn cleanup_terminal(stdout: &mut std::io::Stdout) {
    let _ = execute!(
        stdout,
        Clear(ClearType::All),
        SetForegroundColor(Color::Reset),
        Show,
        LeaveAlternateScreen
    );
    let _ = disable_raw_mode();
}

pub fn run_matrix(
    initial_drops: usize,
    use_rgb_fade: bool,
    charset: &[char],
    fps: u32,
) -> std::io::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let stuck_chars = Rc::new(RefCell::new(HashMap::<u16, (u16, char)>::new()));

    let stuck_chars_for_handler = stuck_chars.clone();

    STUCK_HANDLER.with(|handler| {
        *handler.borrow_mut() = Box::new(move |x, y, ch| {
            let limit = get_stuck_max();
            let mut map = stuck_chars_for_handler.borrow_mut();
            if limit == 0 || map.len() < limit {
                map.insert(x, (y, ch));
            }
        });
    });

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    set_framerate(fps as f32);

    let mut stdout = stdout();
    enable_raw_mode()?;
    let (mut cols, mut rows) = size()?;
    let mut rng = thread_rng();
    let mut drops: Vec<Option<MatrixDrop>> = vec![None; cols as usize];

    let mut columns: Vec<u16> = (0..cols).collect();
    columns.shuffle(&mut rng);

    for &col in columns.iter().take(initial_drops.min(cols as usize)) {
        drops[col as usize] = Some(MatrixDrop::new(col, rows, charset));
    }

    execute!(stdout, EnterAlternateScreen, Hide, Clear(ClearType::All))?;
    let mut last_spawn_check = Instant::now();

    loop {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        if poll(Duration::from_millis(1))? {
            match read()? {
                Event::Key(key) => match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Char('Q'), _) | (KeyCode::Esc, _) => break,
                    (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => break,
                    _ => {}
                },
                Event::Resize(new_cols, new_rows) => {
                    cols = new_cols;
                    rows = new_rows;
                    stuck_chars.borrow_mut().clear();
                    execute!(stdout, Clear(ClearType::All))?;
                    drops = (0..cols)
                        .map(|x| if rng.r#gen::<f32>() < 0.3 {
                            Some(MatrixDrop::new(x, rows, charset))
                        } else {
                            None
                        })
                        .collect();
                }
                _ => {}
            }
        }

        if last_spawn_check.elapsed().as_secs_f32() > 0.2 {
            for (x, drop_slot) in drops.iter_mut().enumerate() {
                if drop_slot.is_none() && rng.r#gen::<f32>() < get_new_drop_probability() {
                    *drop_slot = Some(MatrixDrop::new(x as u16, rows, charset));
                }
            }
            last_spawn_check = Instant::now();
        }

        for (&x, &(y, ch)) in stuck_chars.borrow().iter() {
            if y < rows {
                queue!(stdout, MoveTo(x, y), SetForegroundColor(MATRIX_GREEN_DIM), Print(ch))?;
            }
        }

        for drop in drops.iter_mut().flatten() {
            drop.update(rows);
            let mut stuck_chars_mut = stuck_chars.borrow_mut();
            drop.render(&mut stdout, rows, use_rgb_fade, &mut *stuck_chars_mut)?;
        }

        stdout.flush()?;
        sleep(BASE_FRAME_DELAY);
    }

    cleanup_terminal(&mut stdout);
    println!("Goodbye from the Matrix...");
    Ok(())
}

// ==== Config Access ====
pub fn set_framerate(fps: f32) {
    FRAMERATE.store(fps.clamp(1.0, 15.0).to_bits(), Ordering::Relaxed);
}
pub fn get_framerate() -> f32 {
    f32::from_bits(FRAMERATE.load(Ordering::Relaxed))
}
pub fn set_glitch_probability(p: f32) {
    GLITCH_PROBABILITY_ATOMIC.store(p.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
}
pub fn get_glitch_probability() -> f32 {
    f32::from_bits(GLITCH_PROBABILITY_ATOMIC.load(Ordering::Relaxed))
}
pub fn set_flicker_probability(p: f32) {
    FLICKER_PROBABILITY_ATOMIC.store(p.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
}
pub fn get_flicker_probability() -> f32 {
    f32::from_bits(FLICKER_PROBABILITY_ATOMIC.load(Ordering::Relaxed))
}
pub fn set_min_trail(v: usize) {
    MIN_TRAIL_ATOMIC.store(v.clamp(TRAIL_MIN_LIMIT, get_max_trail()).try_into().unwrap(), Ordering::Relaxed);
}
pub fn get_min_trail() -> usize {
    MIN_TRAIL_ATOMIC.load(Ordering::Relaxed) as usize
}
pub fn set_max_trail(v: usize) {
    MAX_TRAIL_ATOMIC.store(v.clamp(get_min_trail(), TRAIL_MAX_LIMIT).try_into().unwrap(), Ordering::Relaxed);
}
pub fn get_max_trail() -> usize {
    MAX_TRAIL_ATOMIC.load(Ordering::Relaxed) as usize
}
pub fn set_new_drop_probability(p: f32) {
    NEW_DROP_PROBABILITY_ATOMIC.store(p.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
}
pub fn get_new_drop_probability() -> f32 {
    f32::from_bits(NEW_DROP_PROBABILITY_ATOMIC.load(Ordering::Relaxed))
}
pub fn set_stuck_probability(p: f32) {
    STUCK_PROBABILITY_ATOMIC.store(p.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
}
pub fn get_stuck_probability() -> f32 {
    f32::from_bits(STUCK_PROBABILITY_ATOMIC.load(Ordering::Relaxed))
}

pub fn set_stuck_max(v: usize) {
    STUCK_MAX_ATOMIC.store(v as u32, Ordering::Relaxed);
}

pub fn get_stuck_max() -> usize {
    STUCK_MAX_ATOMIC.load(Ordering::Relaxed) as usize
}
