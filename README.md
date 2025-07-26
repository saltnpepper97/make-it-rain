# 🌧️ Make It Rain - Terminal Matrix Effect

<p align="center">
  <img src="assets/mir.gif" alt="Demo" />
</p>

A fast, customizable **Matrix rain animation** for your terminal written in Rust. Featuring smooth trails, RGB mode, drop glitching, and full control over the look and feel.

---

## ✨ Features


- Smooth falling Matrix drops with variable speed and trail length  
- RGB fade or classic green shading  
- Glitching and flickering effects for dynamic visuals  
- Stuck characters left behind by drops, controllable or disableable  
- Configurable frame rate and character palettes  
- Built with `crossterm` for fast terminal rendering

---

## ⚙️ Options
```
  -D, --debug                        Enable debug output
  -n, --drops <DROPS>                Initial number of active drops [default: 10]
      --rgb                          Enable RGB fade coloring instead of preset green steps
      --min-trail <MIN_TRAIL>        Minimum trail length (clamped between 4 and 40, cannot exceed --max-trail) [default: 8]
      --max-trail <MAX_TRAIL>        Maximum trail length (clamped between 4 and 40, cannot be less than --min-trail) [default: 25]
      --glitch-prob <GLITCH_PROB>    Probability of glitch characters appearing (0.0 - 1.0) [default: 0.003]
      --flicker-prob <FLICKER_PROB>  Probability of character flickering (0.0 - 1.0) [default: 0.01]
      --stuck-prob <STUCK_PROB>      Probability (0.0–1.0) that a falling drop leaves a character stuck on screen when it resets. Lower = fewer stuck characters [default: 0.02]
      --drop-prob <DROP_PROB>        Probability of a new drop spawning in an empty column (0.0 - 1.0) [default: 0.05]
      --fps <FPS>                    Frames per second (clamped between 1 and 15) [default: 12]
      --palette <PALETTE>            Character palette to use: classic | katakana | alphanumeric | symbols | greek [default: classic]
      --no-stuck                     Disable stuck characters (characters remain after drop moves)
      --no-glitch                    Disable glitch effects entirely
      --no-flicker                   Disable flickering effects entirely
  -h, --help                         Print help
  -V, --version                      Print version
```

---

## 🚀 Installation

Make sure you have Rust installed. Then:

```bash
git clone https://github.com/yourusername/make-it-rain.git
cd make-it-rain
cargo build --release
