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

The dashboard displays the voice avatar alongside alarm, countdown, and stopwatch tiles. Press **F** for a larger avatar view. The avatar fits its panel without cropping. **Up/Down** adjusts the microphone noise gate by 2 dB, **R** reloads images, and **Esc** quits. The gate starts at -45 dBFS; increase it if background noise triggers the avatar or decrease it if quiet speech is ignored. Talking starts 4 dB above the gate, loud talking 24 dB above it. Smoothing and hysteresis prevent flickering on short pauses.

Full-scene anti-aliasing uses 4-sample supersampling: the avatar and text render at twice the window's width and height before being smoothly downsampled. Avatar textures also use mipmaps and trilinear filtering. The render buffer follows window resizing; this avoids requesting a window MSAA mode that can fail on some GLX drivers.

Text, controls, and avatar margins scale together from a 1000×820 dashboard. Wide and tall windows use centered letterboxing, so controls and mouse hit areas stay aligned. The minimum window size is 720×600.

Linux needs ALSA + X11/GL dev packages (`libasound2-dev libx11-dev libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev libgl1-mesa-dev`) plus `cmake` and `clang`.

See [MANUAL.md](MANUAL.md) for details. Run `cargo test` to test voice states, timekeeping, launch modes, and layout.

## Alarm, timer, and stopwatch

- **Alarm:** click the time field, use Ctrl+A to clear, and enter local 24-hour `HH:MM`. Click **Arm** (or Enter) to schedule the next occurrence. **Cancel** disarms it. When it rings, use **Dismiss** or **Snooze 5m**. Alarms are one-shot; re-arm for another day.
- **Timer:** enter `MM:SS` or `HH:MM:SS`, then **Start**, **Pause**, or **Resume**. **Reset** clears the countdown and unlocks the duration field. Completion rings once and stays visible until dismissed or reset.
- **Stopwatch:** **Start**, **Pause**, **Lap**, and **Reset**. Laps show cumulative elapsed time; the latest four remain visible.
- **A / T / S** or the top buttons hide/reopen each time tile. **F** toggles avatar focus. Hidden tools keep running.
- Alarm and timer completion show a banner and play a repeating tone. **Sound: on/muted** controls audio independently of grid launching. The app remains usable without a microphone or output device; audio failures are printed in the terminal.

Timers and stopwatch use monotonic elapsed time, so rendering delays and wall-clock corrections do not distort their timing. Alarms follow the system's local wall clock. Times skipped by a daylight-saving transition use the next valid day; repeated times use the next future occurrence.

**Keep the application open:** tools and targets are session-only, are not restored after exit, and do not wake a sleeping computer. Monotonic-clock behavior during system suspend depends on the operating system.

## Open or run a Selenite grid piece

Drop one Selenite grid (`.json`) or game script (`.rb`) into the window, or provide `--target`. Enable **Launch grid** separately on the desired tools:

- Alarm: opens/runs the target when the alarm fires, including after snooze.
- Timer: opens/runs the target when the countdown finishes.
- Stopwatch: opens/runs the target on a fresh start; resuming a paused stopwatch does not launch it again. Reset before another fresh start.

A JSON grid opens in Selenite with `--grid`; a Ruby game runs with `--game` (requires a Selenite build with scripting). Merely selecting a target does not execute it. All enabled tools share the currently selected target; **Clear target** removes it. Each event launches a separate Selenite process, with at most eight still-open instances launched by this app. Closing mic_freq leaves those windows open. Launch errors appear in the dashboard and child output goes to the terminal.

By default, `selenite` must be on PATH. Supply `--selenite` when using a local build or the portable launcher:

```sh
cargo run --release -- --selenite ../dist/run.sh --target /path/to/grid.json
cargo run --release -- /path/to/avatar-images --selenite ../dist/run.sh --target /path/to/game.rb
cargo run -- --help
```

Target paths are passed as individual process arguments; paths containing spaces work without shell commands. The selected path and launch toggles can be changed while time tools run; the values at the moment of the event are used.
