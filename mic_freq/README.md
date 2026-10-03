# mic_freq

Voice-reactive VTuber-style avatar built with **raylib** and **cpal**. The microphone's volume switches between one of three full-frame avatar images: idle, talking and loud talking. No speech recognition, internet connection or frequency matching is involved.

## Run

Put three images named `idle.png`, `talking.png` and `loud.png` in an `assets/` folder **beside the executable**. For `cargo run --release`, that folder is `target/release/assets/`:

```sh
cd mic_freq
cargo run --release
cargo run --release -- /path/to/images  # optional custom image folder
```

When distributing the built executable, copy the `assets/` folder into the same directory as the executable. An explicit image-folder argument overrides the default; relative arguments are resolved from the current working directory.

PNG, JPG/JPEG, BMP, GIF (first frame), QOI and TGA are supported. Use transparent PNGs for avatars with transparent backgrounds in the image itself. Missing images show colored placeholders and a warning in the terminal; press **R** after adding or changing files to reload. Only one image per state is allowed.

The window displays one image at a time, centered and scaled to fit without cropping. **Up/Down** adjusts the microphone noise gate by 2 dB, **R** reloads images, and **Esc** quits. The gate starts at -45 dBFS; increase it if background noise triggers the avatar or decrease it if quiet speech is ignored. Talking starts 4 dB above the gate, loud talking 24 dB above it. Smoothing and hysteresis prevent flickering on short pauses.

Linux needs ALSA + X11/GL dev packages (`libasound2-dev libx11-dev libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev libgl1-mesa-dev`) plus `cmake` and `clang`.

See [MANUAL.md](MANUAL.md) for details. Run `cargo test` to test the voice-state logic.
