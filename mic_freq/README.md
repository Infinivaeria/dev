# mic_freq

Real-time microphone pitch tracker built with **raylib** (rendering), **cpal** (mic capture) and **rustfft** (spectrum analysis). The dominant frequency of the incoming sound drives a "level" on each image asset.

## Run

```sh
cargo run --release              # uses ./assets (falls back to generated placeholder images)
cargo run --release -- my/images # any folder of png/jpg/bmp/gif/qoi/tga
```

Linux needs ALSA + X11/GL dev packages (`libasound2-dev libx11-dev libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev libgl1-mesa-dev`) plus `cmake` and `clang`.

Controls: **Tab** opens the image editor, **Ctrl+S** saves, **Up/Down** raises or lowers the noise gate, and **Esc** closes the editor or quits.

## Customizing images

The **Tab** editor panel (raygui) lets you rename, hide, reorder, remove, recolor and retune every image. For each image you can set the frequency, band width, sensitivity, size, idle size and opacity, lift, and wobble. Click an image on the stage to select it. Drag & drop image files onto the window to add them (or to replace the selected one). Settings are saved to `mic_freq.json` in the image folder.

## How it works

1. `audio.rs` opens the default input device, mixes it down to mono and stores the newest samples in a ring buffer.
2. Each frame, `analysis.rs` applies a Hann window to the latest 4096 samples, runs an FFT, finds the strongest peak between 50 and 4000 Hz, and refines it to less than one bin with Gaussian interpolation. If the input is below the gate (-50 dBFS by default), no frequency is reported.
3. `config.rs` loads `mic_freq.json` (per-image settings, order and ignored files) and reconciles it with the folder. New files are sorted by name and get band centers spaced on a log scale from 100 to 1600 Hz.
4. Each image's target level is `band_affinity(freq, center, width)` (a Gaussian) multiplied by loudness × sensitivity. Levels rise quickly and fall back slowly.
5. Each level changes the image's size, opacity, height and wobble (per-image ranges). It's also shown as a discrete step from `L0` to `L5`.

See [MANUAL.md](MANUAL.md) for full usage, the editor, image naming and tuning.

Global constants (FFT size, steps, attack/release) are at the top of `src/main.rs`.

## Test

```sh
cargo test --lib
```
