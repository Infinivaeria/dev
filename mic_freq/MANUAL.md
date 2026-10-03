# mic_freq — Voice avatar manual

## Requirements

- A working default microphone input device.
- Rust 1.88+, `cmake`, `clang`; on Linux: `libasound2-dev libx11-dev libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev libgl1-mesa-dev`.

## Avatar images

Place `idle.png`, `talking.png`, and `loud.png` in an `assets/` folder in the **same directory as the executable**. When running `cargo run --release`, use `mic_freq/target/release/assets/`; for a standalone executable, distribute its `assets/` folder alongside it. You can use another folder by passing its path as the first argument to `cargo run --release -- /path/to/images`. A relative argument is resolved from the current working directory, not the executable directory.

Each image is a complete avatar pose, not a part to be overlaid. The program shows only one at a time, scaled to fit the window and preserving its aspect ratio. Transparent PNGs work well for avatar artwork. Supported extensions: png, jpg/jpeg, bmp, gif (first frame), qoi and tga. Names and extensions are case-insensitive. Keep only one matching image per state (e.g. do not have both `idle.png` and `idle.jpg`).

If a state has no image, a colored placeholder appears and the terminal prints a warning. Press **R** to reload images after editing them. Invalid or duplicate image files produce an error; on reload the current images stay in use and the error is shown in the window.

## Controls

| Key | Action |
|-----|--------|
| Up / Down | Raise / lower the noise gate by 2 dB |
| R | Reload avatar images |
| Esc | Quit |

The default gate is -45 dBFS. A talking pose appears when microphone volume exceeds the gate by 4 dB; the loud pose appears 24 dB above it. The talking pose returns to idle below the gate, and the loud pose returns to talking below gate + 20 dB. Volume smoothing reduces frame-to-frame flicker. The current state, microphone level and gate are shown at the top of the window.

The image choice depends on microphone *loudness*, not the words you say or the pitch of your voice. Speech is not recorded or transcribed.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| No default input device | Set a working microphone as your system's default input. |
| Always idle | Lower the gate with **Down** or raise microphone gain. |
| Always talking | Raise the gate with **Up** to filter background noise. |
| Colored placeholders | Check the asset folder and filenames, then press **R**. |
| Reload error | Check image format and remove duplicate state images; the terminal contains the full error. |
