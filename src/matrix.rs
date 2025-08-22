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
    collections::HashMap,
    io::{stdout, Write},
    sync::{
        atomic::{AtomicU32, AtomicBool, Ordering},
        Arc,
    },
    thread::sleep,
    time::{Duration, Instant},
};

pub use crate::colors::{MatrixColorScheme, fade_color_rgb};

// ==== Visual Character Sets ====
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

// ==== Animation Configuration ====
static MIN_TRAIL_ATOMIC: AtomicU32 = AtomicU32::new(8);
static MAX_TRAIL_ATOMIC: AtomicU32 = AtomicU32::new(25);
const TRAIL_MIN_LIMIT: usize = 4;
const TRAIL_MAX_LIMIT: usize = 40;
const BASE_FRAME_DELAY: Duration = Duration::from_millis(60);
static FRAMERATE: AtomicU32 = AtomicU32::new((12.0f32).to_bits());
const SPEED_VARIATION: f32 = 0.3;

// ==== Probability Configuration ====
const CHAR_CHANGE_PROBABILITY: f32 = 0.2;
static GLITCH_PROBABILITY_ATOMIC: AtomicU32 = AtomicU32::new((0.003_f32).to_bits());
static FLICKER_PROBABILITY_ATOMIC: AtomicU32 = AtomicU32::new((0.01_f32).to_bits());
static NEW_DROP_PROBABILITY_ATOMIC: AtomicU32 = AtomicU32::new((0.05_f32).to_bits());
static STUCK_PROBABILITY_ATOMIC: AtomicU32 = AtomicU32::new((0.02_f32).to_bits());
const SPEED_JITTER_PROBABILITY: f64 = 0.02;
const SPEED_JITTER_AMOUNT: f32 = 0.05;

/// A falling Matrix-style character drop
#[derive(Clone)]
pub struct MatrixDrop<'a> {
    x: u16,
    y: f32,
    prev_y: f32,
    length: usize,
    speed: f32,
    chars: Vec<char>,
    last_update: Instant,
    charset: &'a [char],
}

impl<'a> MatrixDrop<'a> {
    /// Create a new Matrix drop at the given column
    pub fn new(x: u16, _rows: u16, charset: &'a [char]) -> Self {
        let mut rng = thread_rng();
        let length = rng.gen_range(get_min_trail()..=get_max_trail());
        let speed = 1.0 + rng.r#gen::<f32>() * SPEED_VARIATION;

        let chars: Vec<char> = (0..length)
            .map(|_| *charset.choose(&mut rng).unwrap())
            .collect();

        Self {
            x,
            y: -(length as f32),
            prev_y: -(length as f32),
            length,
            speed,
            chars,
            last_update: Instant::now(),
            charset,
        }
    }

    /// Update the drop's position and characters
    /// Returns true if the drop should be reset
    pub fn update(&mut self, rows: u16) -> bool {
        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32();
        let fps = get_framerate();
        
        self.prev_y = self.y;
        self.y += self.speed * dt * fps;
        self.last_update = now;

        // Add some random speed variation
        let mut rng = thread_rng();
        if rng.gen_bool(SPEED_JITTER_PROBABILITY) {
            let delta = rng.gen_range(-SPEED_JITTER_AMOUNT..SPEED_JITTER_AMOUNT);
            self.speed = (self.speed + delta).clamp(0.5, 3.0);
        }

        // Check if drop has moved off screen
        if self.y > rows as f32 + self.length as f32 {
            return true; // Signal that this drop should be recreated
        }

        // Update character changes
        for ch in &mut self.chars {
            if rng.r#gen::<f32>() < CHAR_CHANGE_PROBABILITY {
                *ch = if rng.gen_bool(0.005) {
                    *GLITCH_CHARS.choose(&mut rng).unwrap()
                } else {
                    *self.charset.choose(&mut rng).unwrap()
                };
            }
        }

        false // Drop is still active
    }

    /// Render the drop to the terminal
    pub fn render(
        &self, 
        w: &mut impl Write, 
        rows: u16, 
        use_rgb_fade: bool, 
        color_scheme: MatrixColorScheme,
        sticky_chars: &mut HashMap<(u16, u16), (char, Instant)>
    ) -> std::io::Result<()> {
        // Clear the previous tail, but check for sticky characters first
        let old_tail_y = (self.prev_y - self.length as f32).floor() as i32;
        let new_tail_y = (self.y - self.length as f32).floor() as i32;
        
        // Clear positions between old and new tail, except sticky ones
        let clear_start = old_tail_y.min(new_tail_y);
        let clear_end = old_tail_y.max(new_tail_y);
        
        for y in clear_start..=clear_end {
            if y >= 0 && (y as u16) < rows {
                let pos = (self.x, y as u16);
                if !sticky_chars.contains_key(&pos) {
                    queue!(w, MoveTo(self.x, y as u16), Print(' '))?;
                }
            }
        }

        // Get color scheme colors
        let (bright, mid, dim, dark, darkest) = color_scheme.get_colors();

        // Render current drop characters
        for (i, &ch) in self.chars.iter().enumerate() {
            let char_y = self.y - i as f32;
            if char_y < 0.0 || char_y >= rows as f32 {
                continue;
            }

            let flicker = thread_rng().gen_bool(get_flicker_probability() as f64);
            let glitch = thread_rng().gen_bool(get_glitch_probability() as f64);

            let color = if use_rgb_fade {
                if i == 0 {
                    bright
                } else {
                    let base_rgb = color_scheme.get_base_rgb();
                    let alpha = 1.0 - (i as f32 / self.length as f32).powf(1.3);
                    fade_color_rgb(base_rgb, alpha)
                }
            } else {
                match i {
                    0 => bright,
                    1..=3 => mid,
                    4..=8 => dim,
                    9..=15 => dark,
                    _ => darkest,
                }
            };

            let display_char = if glitch {
                *GLITCH_CHARS.choose(&mut thread_rng()).unwrap_or(&ch)
            } else if flicker {
                ' '
            } else {
                ch
            };

            let pos = (self.x, char_y as u16);
            // Remove any sticky character at this position (drop overwrites it)
            sticky_chars.remove(&pos);
            
            queue!(w, MoveTo(self.x, char_y as u16), SetForegroundColor(color), Print(display_char))?;
        }

        Ok(())
    }

    /// Check if this drop should leave a stuck character when it resets
    pub fn should_leave_sticky(&self, rows: u16) -> Option<(u16, u16, char)> {
        if self.y > rows as f32 + self.length as f32 {
            let mut rng = thread_rng();
            if rng.r#gen::<f32>() < get_stuck_probability() {
                // Pick the last character and a random position on screen
                if let Some(&last_char) = self.chars.last() {
                    let stick_y = rng.gen_range(0..rows);
                    return Some((self.x, stick_y, last_char));
                }
            }
        }
        None
    }
}

/// Clean up the terminal state on exit
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

/// Main Matrix effect runner
pub fn run_matrix(
    initial_drops: usize,
    use_rgb_fade: bool,
    charset: &[char],
    fps: u32,
    enable_stuck: bool,
    color_scheme: MatrixColorScheme,
) -> std::io::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Set up Ctrl+C handler
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    set_framerate(fps as f32);

    let mut stdout = stdout();
    enable_raw_mode()?;
    let (mut cols, mut rows) = size()?;
    let mut rng = thread_rng();
    let mut drops: Vec<Option<MatrixDrop>> = vec![None; cols as usize];
    let mut sticky_chars: HashMap<(u16, u16), (char, Instant)> = HashMap::new();

    // Randomize initial drop positions
    let mut columns: Vec<u16> = (0..cols).collect();
    columns.shuffle(&mut rng);

    for &col in columns.iter().take(initial_drops.min(cols as usize)) {
        drops[col as usize] = Some(MatrixDrop::new(col, rows, charset));
    }

    execute!(stdout, EnterAlternateScreen, Hide, Clear(ClearType::All))?;
    let mut last_spawn_check = Instant::now();

    // Main animation loop
    'main: loop {
        if !running.load(Ordering::SeqCst) {
            break 'main;
        }

        // Handle input events
        if poll(Duration::from_millis(1))? {
            match read()? {
                Event::Key(key) => match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Char('Q'), _) | (KeyCode::Esc, _) => break 'main,
                    (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => break 'main,
                    _ => {}
                },
                Event::Resize(new_cols, new_rows) => {
                    cols = new_cols;
                    rows = new_rows;
                    sticky_chars.clear();
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

        // Clean up old stuck characters (remove after 10 seconds)
        if enable_stuck {
            let now = Instant::now();
            sticky_chars.retain(|_, (_, timestamp)| {
                now.duration_since(*timestamp).as_secs() < 10
            });
        }

        // Spawn new drops periodically
        let now = Instant::now();
        if now.duration_since(last_spawn_check).as_secs_f32() > 0.2 {
            for (x, drop_slot) in drops.iter_mut().enumerate() {
                if drop_slot.is_none() && rng.r#gen::<f32>() < get_new_drop_probability() {
                    *drop_slot = Some(MatrixDrop::new(x as u16, rows, charset));
                }
            }
            last_spawn_check = now;
        }

        // Get stuck character color
        let (_, _, stuck_color, _, _) = color_scheme.get_colors();

        // Render stuck characters first (so drops can overwrite them)
        if enable_stuck {
            for (&(x, y), &(ch, _)) in sticky_chars.iter() {
                if y < rows {
                    queue!(stdout, MoveTo(x, y), SetForegroundColor(stuck_color), Print(ch))?;
                }
            }
        }

        // Update and render drops
        for drop_slot in drops.iter_mut() {
            if let Some(drop) = drop_slot {
                let should_reset = drop.update(rows);
                
                // Check if drop should leave a stuck character before resetting
                if enable_stuck && should_reset {
                    if let Some((x, y, ch)) = drop.should_leave_sticky(rows) {
                        sticky_chars.insert((x, y), (ch, Instant::now()));
                    }
                }
                
                if should_reset {
                    *drop = MatrixDrop::new(drop.x, rows, charset);
                } else {
                    drop.render(&mut stdout, rows, use_rgb_fade, color_scheme, &mut sticky_chars)?;
                }
            }
        }

        stdout.flush()?;
        sleep(BASE_FRAME_DELAY);
    }

    cleanup_terminal(&mut stdout);
    println!("Goodbye from the Matrix...");
    Ok(())
}

// ==== Configuration Getters and Setters ====

pub fn set_framerate(fps: f32) {
    let fps = fps.clamp(1.0, 15.0);
    FRAMERATE.store(fps.to_bits(), Ordering::Relaxed);
}

pub fn get_framerate() -> f32 {
    f32::from_bits(FRAMERATE.load(Ordering::Relaxed))
}

pub fn set_glitch_probability(prob: f32) {
    let prob = prob.clamp(0.0, 1.0);
    GLITCH_PROBABILITY_ATOMIC.store(prob.to_bits(), Ordering::Relaxed);
}

pub fn get_glitch_probability() -> f32 {
    f32::from_bits(GLITCH_PROBABILITY_ATOMIC.load(Ordering::Relaxed))
}

pub fn set_flicker_probability(prob: f32) {
    let prob = prob.clamp(0.0, 1.0);
    FLICKER_PROBABILITY_ATOMIC.store(prob.to_bits(), Ordering::Relaxed);
}

pub fn get_flicker_probability() -> f32 {
    f32::from_bits(FLICKER_PROBABILITY_ATOMIC.load(Ordering::Relaxed))
}

pub fn set_stuck_probability(prob: f32) {
    let prob = prob.clamp(0.0, 1.0);
    STUCK_PROBABILITY_ATOMIC.store(prob.to_bits(), Ordering::Relaxed);
}

pub fn get_stuck_probability() -> f32 {
    f32::from_bits(STUCK_PROBABILITY_ATOMIC.load(Ordering::Relaxed))
}

pub fn set_min_trail(len: usize) {
    let clamped = len.clamp(TRAIL_MIN_LIMIT, TRAIL_MAX_LIMIT.min(get_max_trail()));
    MIN_TRAIL_ATOMIC.store(clamped as u32, Ordering::Relaxed);
}

pub fn get_min_trail() -> usize {
    MIN_TRAIL_ATOMIC.load(Ordering::Relaxed) as usize
}

pub fn set_max_trail(len: usize) {
    let clamped = len.clamp(get_min_trail().max(TRAIL_MIN_LIMIT), TRAIL_MAX_LIMIT);
    MAX_TRAIL_ATOMIC.store(clamped as u32, Ordering::Relaxed);
}

pub fn get_max_trail() -> usize {
    MAX_TRAIL_ATOMIC.load(Ordering::Relaxed) as usize
}

pub fn set_new_drop_probability(prob: f32) {
    let prob = prob.clamp(0.0, 1.0);
    NEW_DROP_PROBABILITY_ATOMIC.store(prob.to_bits(), Ordering::Relaxed);
}

pub fn get_new_drop_probability() -> f32 {
    f32::from_bits(NEW_DROP_PROBABILITY_ATOMIC.load(Ordering::Relaxed))
}
