use crossterm::style::Color;

/// Color scheme configuration for the Matrix effect
#[derive(Clone, Copy, Debug)]
pub enum MatrixColorScheme {
    Green,
    Custom(Color),
}

impl MatrixColorScheme {
    /// Create a color scheme from an ANSI color code (0-15)
    pub fn from_ansi_code(code: u8) -> Self {
        let color = match code {
            0 => Color::Black,
            1 => Color::DarkRed,
            2 => Color::DarkGreen,
            3 => Color::DarkYellow,
            4 => Color::DarkBlue,
            5 => Color::DarkMagenta,
            6 => Color::DarkCyan,
            7 => Color::Grey,
            8 => Color::DarkGrey,
            9 => Color::Red,
            10 => Color::Green,
            11 => Color::Yellow,
            12 => Color::Blue,
            13 => Color::Magenta,
            14 => Color::Cyan,
            15 => Color::White,
            _ => Color::Green, // Default to green for invalid codes
        };
        
        // Use the classic green scheme for green colors, otherwise use custom
        if matches!(color, Color::Green | Color::DarkGreen) {
            Self::Green
        } else {
            Self::Custom(color)
        }
    }
    
    /// Get the five-color gradient for this scheme
    /// Returns: (bright_head, mid, dim, dark, darkest)
    pub fn get_colors(self) -> (Color, Color, Color, Color, Color) {
        match self {
            Self::Green => (
                Color::White,      // Bright head (always white for visibility)
                Color::Green,      // Mid green
                Color::DarkGreen,  // Dim green
                Color::DarkGreen,  // Dark green
                Color::Black,      // Darkest
            ),
            Self::Custom(base_color) => {
                // For custom colors, create a fade effect
                // Head is always white for visibility, then fade through the base color
                (Color::White, base_color, base_color, base_color, Color::Black)
            }
        }
    }
    
    /// Get RGB components for fade calculations
    pub fn get_base_rgb(self) -> (u8, u8, u8) {
        match self {
            Self::Green => (0, 255, 0),
            Self::Custom(color) => {
                // For custom colors in RGB fade mode, approximate the RGB values
                match color {
                    Color::Red => (255, 0, 0),
                    Color::DarkRed => (139, 0, 0),
                    Color::Blue => (0, 0, 255),
                    Color::DarkBlue => (0, 0, 139),
                    Color::Yellow => (255, 255, 0),
                    Color::DarkYellow => (184, 134, 11),
                    Color::Magenta => (255, 0, 255),
                    Color::DarkMagenta => (139, 0, 139),
                    Color::Cyan => (0, 255, 255),
                    Color::DarkCyan => (0, 139, 139),
                    Color::White => (255, 255, 255),
                    Color::Grey => (192, 192, 192),
                    Color::DarkGrey => (169, 169, 169),
                    Color::Black => (0, 0, 0),
                    _ => (0, 255, 0), // Default to green
                }
            }
        }
    }
}

/// Create a faded RGB color
pub fn fade_color_rgb((r, g, b): (u8, u8, u8), alpha: f32) -> Color {
    Color::Rgb {
        r: (r as f32 * alpha).clamp(0.0, 255.0) as u8,
        g: (g as f32 * alpha).clamp(0.0, 255.0) as u8,
        b: (b as f32 * alpha).clamp(0.0, 255.0) as u8,
    }
}
