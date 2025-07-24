use clap::Parser;

mod matrix;

#[derive(Parser, Debug)]
#[command(name = "mir")]
#[command(about = "Make-it-Rain: Matrix rain effect", long_about = None)]
struct Cli {
    #[arg(short = 'D', long, help = "Enable debug output")]
    debug: bool,

    #[arg(short = 'n', long, default_value_t = 10, help = "Initial number of active drops")]
    drops: usize,

    #[arg(long, help = "Enable RGB fade coloring instead of preset green steps")]
    rgb: bool,

    #[arg(
        long,
        default_value_t = 8,
        help = "Minimum trail length (clamped between 4 and 40, cannot exceed --max-trail)"
    )]
    min_trail: usize,

    #[arg(
        long,
        default_value_t = 25,
        help = "Maximum trail length (clamped between 4 and 40, cannot be less than --min-trail)"
    )]
    max_trail: usize,

    #[arg(
        long,
        default_value_t = 0.003,
        help = "Probability of glitch characters appearing (0.0 - 1.0)"
    )]
    glitch_prob: f64,

    #[arg(
        long,
        default_value_t = 0.01,
        help = "Probability of character flickering (0.0 - 1.0)"
    )]
    flicker_prob: f64,

    #[arg(
        long,
        default_value_t = 0.05,
        help = "Probability of a new drop spawning in an empty column (0.0 - 1.0)"
    )]
    drop_prob: f32,

    #[arg(
        long,
        default_value_t = 12,
        help = "Frames per second (clamped between 1 and 15)"
    )]
    fps: u32,

    #[arg(
        long,
        default_value = "classic",
        help = "Character palette to use: classic | katakana | alphanumeric | symbols | greek"
    )]
    palette: String,

    #[arg(long, help = "Disable glitch effects entirely")]
    no_glitch: bool,

    #[arg(long, help = "Disable flickering effects entirely")]
    no_flicker: bool,
}


fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    if cli.debug {
        eprintln!("Debug mode enabled");
    }

    // Set glitch and flicker probabilities based on CLI flags and values
    let glitch_prob = if cli.no_glitch { 0.0 } else { cli.glitch_prob as f32 };
    let flicker_prob = if cli.no_flicker { 0.0 } else { cli.flicker_prob as f32 };

    // Set values before launching
    matrix::set_glitch_probability(glitch_prob);
    matrix::set_flicker_probability(flicker_prob);
    matrix::set_min_trail(cli.min_trail);
    matrix::set_max_trail(cli.max_trail);
    matrix::set_new_drop_probability(cli.drop_prob);
    matrix::set_framerate(cli.fps as f32);

    // Prebuild combined charset vector once
    let combined_charset: Vec<char> = {
        let mut v = Vec::new();
        v.extend_from_slice(matrix::MATRIX_CHARS_KATAKANA);
        v.extend_from_slice(matrix::MATRIX_CHARS_ALPHANUMERIC);
        v.extend_from_slice(matrix::MATRIX_CHARS_SYMBOLS);
        v.extend_from_slice(matrix::MATRIX_CHARS_GREEK);
        v
    };

    // Determine charset from palette string
    let charset: &[char] = match cli.palette.to_lowercase().as_str() {
        "katakana" => matrix::MATRIX_CHARS_KATAKANA,
        "alphanumeric" => matrix::MATRIX_CHARS_ALPHANUMERIC,
        "symbols" => matrix::MATRIX_CHARS_SYMBOLS,
        "greek" => matrix::MATRIX_CHARS_GREEK,
        "classic" | _ => &combined_charset,
    };

    // Run the matrix animation
    matrix::run_matrix(cli.drops, cli.rgb, charset, cli.fps)
}

