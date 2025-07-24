# 🌧️ Make It Rain - Terminal Matrix Effect

![Demo](assets/mir.gif)

A fast, customizable **Matrix rain animation** for your terminal written in Rust. Featuring smooth trails, RGB mode, drop glitching, and full control over the look and feel.

---

## ✨ Features

- 💧 Matrix drop animation with smooth speed variation and dynamic character changes
- 🎨 RGB fade mode or classic green shading (`--rgb`)
- 🎛️ Fully configurable drop behavior:
  - `--drops`: initial number of active drops
  - `--min-trail` / `--max-trail`: control trail length
  - `--drop-prob`: chance for new drops to spawn each frame
  - `--fps`: frames per second (1–15)
- 💥 Optional glitch effect on drops (`--glitch`)
- 🧵 Built using `crossterm` for snappy performance

---

## 🚀 Installation

Make sure you have Rust installed. Then:

```bash
git clone https://github.com/yourusername/make-it-rain.git
cd make-it-rain
cargo build --release