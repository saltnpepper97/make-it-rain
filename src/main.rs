mod matrix;
mod colors;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "mir",
    about = "Make-it-Rain: Matrix rain effect",
    long_about = "A customizable Matrix-style digital rain effect for your terminal.\n\nFor detailed documentation including color codes and examples, see: man mir",
    version = env!("CARGO_PKG_VERSION")
)]
struct Cli {
    #[arg(short = 'D', long, help = "Enable debug output")]
    debug: bool,

    #[arg(short = 'n', long, default_value_t = 10, help = "Initial number of active drops")]
    drops: usize,

    #[arg(long, help = "Enable RGB fade coloring")]
    rgb: bool,

    #[arg(
        short = 'c',
        long = "color",
        default_value_t = 10,
        help = "Terminal color code (0-15)"
    )]
    color: u8,

    #[arg(long, default_value_t = 8, help = "Minimum trail length")]
    min_trail: usize,

    #[arg(long, default_value_t = 25, help = "Maximum trail length")]
    max_trail: usize,

    #[arg(long, default_value_t = 0.003, help = "Glitch probability")]
    glitch_prob: f64,

    #[arg(long, default_value_t = 0.01, help = "Flicker probability")]
    flicker_prob: f64,

    #[arg(long, default_value = "0.02", help = "Stuck character probability")]
    stuck_prob: f32,
    
    #[arg(long, default_value_t = 0.05, help = "New drop probability")]
    drop_prob: f32,

    #[arg(long, default_value_t = 12, help = "Frames per second")]
    fps: u32,

    #[arg(long, default_value = "classic", help = "Character palette")]
    palette: String,

    #[arg(long = "no-stuck", help = "Disable stuck characters")]
    no_stuck: bool,

    #[arg(long, help = "Disable glitch effects")]
    no_glitch: bool,

    #[arg(long, help = "Disable flickering effects")]
    no_flicker: bool,
}

fn get_charset_by_name(name: &str) -> &'static [char] {
    match name.to_lowercase().as_str() {
        "katakana" => matrix::MATRIX_CHARS_KATAKANA,
        "alphanumeric" => matrix::MATRIX_CHARS_ALPHANUMERIC,
        "symbols" => matrix::MATRIX_CHARS_SYMBOLS,
        "greek" => matrix::MATRIX_CHARS_GREEK,
        "classic" | _ => {
            // For "classic" or any unrecognized name, return all combined
            // We need to use a static reference, so we'll use a lazy_static or similar approach
            // For now, let's default to katakana + alphanumeric which are most common
            matrix::MATRIX_CHARS_KATAKANA
        }
    }
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    
    if cli.debug {
        eprintln!("Debug mode enabled");
        eprintln!("Color: {}", cli.color);
        eprintln!("Palette: {}", cli.palette);
        eprintln!("RGB mode: {}", cli.rgb);
        eprintln!("For detailed color reference, see: man mir");
    }

    // Validate color range
    let color_code = if cli.color > 15 {
        eprintln!("Warning: Color code {} is out of range (0-15), using 10 (Green)", cli.color);
        eprintln!("See 'man mir' for all available colors.");
        10
    } else {
        cli.color
    };

    // Set up color scheme
    let color_scheme = matrix::MatrixColorScheme::from_ansi_code(color_code);

    // Set glitch and flicker probabilities based on CLI flags and values
    let glitch_prob = if cli.no_glitch { 0.0 } else { cli.glitch_prob as f32 };
    let flicker_prob = if cli.no_flicker { 0.0 } else { cli.flicker_prob as f32 };
    
    matrix::set_glitch_probability(glitch_prob);
    matrix::set_flicker_probability(flicker_prob);
    matrix::set_min_trail(cli.min_trail);
    matrix::set_max_trail(cli.max_trail);
    matrix::set_new_drop_probability(cli.drop_prob);
    matrix::set_framerate(cli.fps as f32);
    matrix::set_stuck_probability(cli.stuck_prob);

    // Handle charset selection
    let charset: &[char] = if cli.palette.to_lowercase() == "classic" {
        // For classic, create a combined charset
        // Since we need a static reference, we'll use a different approach
        // Let's create a Vec and leak it to get a 'static reference
        let combined_charset: Vec<char> = {
            let mut v = Vec::new();
            v.extend_from_slice(matrix::MATRIX_CHARS_KATAKANA);
            v.extend_from_slice(matrix::MATRIX_CHARS_ALPHANUMERIC);
            v.extend_from_slice(matrix::MATRIX_CHARS_SYMBOLS);
            v.extend_from_slice(matrix::MATRIX_CHARS_GREEK);
            v
        };
        Box::leak(combined_charset.into_boxed_slice())
    } else {
        get_charset_by_name(&cli.palette)
    };

    if cli.debug {
        eprintln!("Selected charset size: {}", charset.len());
    }

    // Run the matrix effect
    matrix::run_matrix(
        cli.drops,
        cli.rgb,
        charset,
        cli.fps,
        !cli.no_stuck,
        color_scheme,
    )
}
