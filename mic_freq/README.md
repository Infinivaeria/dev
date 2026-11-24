# Named Voice Presets

Use the Presets button or F2 to save and load named voice/presentation settings.
Presets keep the active microphone and avatar folder unchanged. Escape closes
the dialog without exiting the studio.

```sh
mic_freq --save-preset "Quiet room"
mic_freq --list-presets
mic_freq --preset "Quiet room"
mic_freq --version
```

Save/list commands do not need a display or microphone. Presets live beside the
studio settings in a `presets` directory.

# Selenite Audio Studio

mic_freq is the **Selenite Microphone Suite**: an offline voice-reactive
PNGTuber studio with live signal analysis and noise gate calibration.
It also includes a sandboxed, Turing-complete
Rhai scripting runtime for live presentation automation. It is built with
raylib and cpal. Microphone volume selects idle,
talking, or loud talking artwork, and detected low/mid/high pitch can select a
different open and closed-eye animation set. The Music Composer and Graphics
Studio are separate global Selenite tools, while Audio Studio focuses on live
voice-driven artwork. It does not require an internet connection.

## Live scripting

Run a script alongside the suite:

```sh
mic_freq --check-script examples/mic_freq-script.rhai
mic_freq --script examples/mic_freq-script.rhai
```

The package includes 30 ready-to-run scripts in `mic_freq/examples/` (also
available from the package's top-level `examples/mic_freq/`), with an index and
usage notes in their README.

Scripts implement `update(input)` and return a map. Input fields expose `frame`,
`time`, `dt`, `db`, `adjusted_db`, `pitch_hz` (`()` when unknown), `band`,
`pose`, `muted`, `gate_db`, and `loud_delta_db`. Optional output fields are
`pose`, `band`, `bounce`, `scale`, `mirror`, `blink`, `gate_db`,
`loud_delta_db`, and `gain_db`. Manual pose selection and mute take priority.

Rhai supplies variables, arithmetic, recursion, functions, loops, arrays and
maps, so scripts are computationally Turing complete. mic_freq bounds operation
counts, nesting, calls, variables and collection/string sizes. Scripts have no
filesystem, process, network or Selenite-setting APIs; they only return
validated presentation overrides for the current run.

## Chiptune Composer

Open the Music Composer from the global Selenite toolbar/suite menu, or launch:

```sh
selenite --composer
selenite --composer song.json
mic_freq --self-test
```

`--self-test` is a headless RC diagnostic. It checks JSON projects, theory,
rendered pitch analysis, WAV round-trip and rhythm tracking without opening a
display or an audio device.

The Composer includes:

- Four tracks, sixteen steps, and a C4-B4 piano roll.
- Sine, square, triangle, saw and deterministic-noise oscillators.
- Default, pluck and pad envelopes plus track volume, pan and mute.
- Major, natural-minor and chromatic scale selection.
- Major, minor, diminished, augmented and seventh chords with inversions.
- I-IV-V-I progression generation, five arpeggiators, and key/chord analysis.
- Explicit output-device selection and synchronized stereo playback.
- JSON project save, WAV/OGG sound assignment, MIDI import and stereo WAV export.

Drop one JSON project to load it, one `.mid`/`.midi` file to import note tracks,
or a WAV/OGG file to assign it as the selected track's sampler source. Audio is
resampled to 44.1 kHz and mixed to stereo. WAV export uses the same deterministic
renderer used for playback.

Projects are saved under `$XDG_CONFIG_HOME/selenite/projects` by default. Save
places the WAV export beside the project. `Ctrl+S` saves, Space starts/stops
playback, arrow keys select the track/step, and Escape closes the Composer.

## Tone, harmonics and rhythm analysis

Audio Studio's scaled **Analyzer** button shows live microphone measurements:

- RMS level, detected fundamental in Hz, nearest note and cents.
- Eight FFT harmonic amplitudes, harmonicity, confidence and brightness.
- Tonal versus percussive balance.
- Onset strength, estimated tempo/BPM and pulse regularity.

The rhythm tracker consumes only newly arrived microphone samples, so normal
display-window overlap does not inflate its tempo estimate. In Composer,
**Analyze Mix** examines the exact deterministic render used by Play and WAV
export. Polyphonic/noisy material can legitimately have no stable single pitch
or tempo; Selenite reports that uncertainty instead of inventing a result.

## Run

The full Selenite extraction package includes this executable, adjacent artwork,
and these guides. After extracting the suite, run `./mic_freq/mic_freq` directly;
the optional `run-mic-freq.sh` wrapper is only for compatibility. Add
`--capture` for OBS or pass your own avatar folder and `--device "NAME"`.
Build that combined package from the repository root with
`./package.sh --output dist-v11 --verify`. The game-only `--game` package does
not include the companion executable. No compilation is required after extracting
the full package; a compatible Linux graphics/audio environment is still needed.

From the repository root:

```sh
cargo build -p mic_freq --release
install -m 755 target/release/mic_freq mic_freq/target/release/mic_freq
install -m 755 target/release/mic_freq mic_freq/target/debug/mic_freq
./target/release/mic_freq mic_freq/assets
cargo run -p mic_freq --release -- /path/to/images
cargo run -p mic_freq -- --list-devices
cargo run -p mic_freq -- --device "USB microphone" --capture
```

Cargo writes the current executable to the workspace's `target/release/mic_freq`. Launch that binary directly; no script or Cargo process is needed to run the built application. The old `mic_freq/target/release/mic_freq` and `mic_freq/target/debug/mic_freq` executables are real, independent copies of the current PNGTuber Studio release build, not symbolic links. After rebuilding, use the `install` commands above to refresh those copies. A running old process must be closed and relaunched to use a new build.

For distribution, copy any of these executables and your `assets/` folder together. No workspace files, launcher script, or symbolic links are required. The destination must be a compatible Linux system with the required audio and graphical system libraries.

Put `idle.png`, `talking.png`, and `loud.png` in your avatar folder. Folder priority is: an explicit folder argument, an existing `assets/` folder beside the executable, then an existing saved folder. Missing saved folders are ignored. This means a copied executable uses its own adjacent artwork instead of an old installation's saved path. The selected directory is printed as `Avatar images: ...` at startup. This workspace builds into the root `target/` directory. Relative folder arguments are resolved from the working directory. Missing artwork uses a transparent, animated demo character, so the app works out of the box.

PNG, JPG/JPEG, BMP, GIF (first frame), QOI, and TGA are supported. Use transparent PNGs and identically sized canvases for consistent framing. Optional `idle_blink.png`, `talking_blink.png`, and `loud_blink.png` provide closed-eye variants; each must match its base pose's dimensions. Missing blink art falls back to the base pose. Only one image per name is allowed, case-insensitively. Click **Artwork** to replace any global open or closed-eye slot by browsing directories or dropping a file.

For pitch-specific animation, add images named `low_idle`, `low_talking`, and
`low_loud` (and the corresponding `mid_` and `high_` names). Each can also have
a `_blink` closed-eye variant, such as `low_talking_blink.png`. All supported
image formats are accepted. An absent pitch pose falls back to the matching
global pose, and an absent pitch blink falls back to that pitch's open pose.
Artwork slots are independent: files with the same original name are copied
under their band-specific names, so editing one band does not replace another.

## Studio controls

- Sliders adjust the noise gate, bounce strength (0-200%), avatar size, loud threshold, and independent Low/Mid/High voice gains. Detected low voices (below 160 Hz), mid voices (160-300 Hz), and high voices (above 300 Hz) receive their selected dB adjustment before the gate and loud threshold are evaluated. The input meter shows the resulting level in dBFS and the detected pitch.
- The **Edit** button beside each Low/Mid/High gain opens an independent frequency-animation window. Configure that pitch band's gain, bounce, avatar size, mirroring and blinking, then select one of the Idle/Talking/Loud open or closed-eye artwork slots and drop an image into it. **Custom animation** starts off so existing global controls continue to apply; turning it on or moving a per-band animation control enables that override. Remove Selected returns an artwork slot to the global pose. Closing the editor reloads its artwork and settings into Audio Studio.
- **Help** or **F1** opens the in-app mic_freq guide. Its PNGTuber box section explains the capture-only mode and every exit shortcut.
- Artwork and frequency editor controls use a fixed logical canvas that scales proportionally with their windows, so labels, buttons, sliders, and browse hit targets remain aligned at any size.
- **Calibrate noise gate** measures three seconds of room noise. Stay quiet during calibration, then test normal speech and adjust as needed.
- The sidebar shows the currently active band and the detected fundamental pitch.
  Voice pitch uses an 80 ms window updated every 50 ms; the separate signal
  analyzer uses its own window and may report a different instantaneous pitch.
- Checkboxes toggle microphone mute, horizontal mirroring, and automatic blinking.
- **Auto / Idle / Talk / Loud** select microphone control or a manual pose. Mute always forces idle.
- **Next input / reconnect** cycles available microphones and the system default. Capture failure does not close the app; manual poses still work.
- Drop one avatar folder or pose image into the window to switch sets. **R** reloads edited artwork. Failed reloads preserve the current set.
- Background swatches select green, magenta, or dark. **PNGTuber box** hides every control, meter, error, and the operating-system title bar/frame. Press **C** or click the canvas once to restore them.

The gate ranges from -120 to 0 dBFS and defaults to -45 dBFS; talking starts 4 dB above it. Loud talking starts 24 dB above it by default, independently adjustable from 4 to 120 dB above the gate. It requires 300 ms of sustained loud input while talking; brief peaks do not trigger it, and idle never jumps directly to loud. Very high settings may be unreachable for your microphone, even with voice gain. Lower the loud threshold if your microphone never reaches the loud pose, or raise it if normal speech triggers loud too easily. Low, Mid and High voice gains each range from -18 to +18 dB, allowing deeper or brighter microphones and voices to use different sensitivities. During speech, a detected band and its gain are retained for up to 300 ms of unpitched input; longer dropouts use the global artwork and no frequency gain. Idle and mute use global artwork and animation. Smoothing and 4 dB hysteresis reduce flicker.

Full-scene anti-aliasing uses 4-sample supersampling: the avatar and text render at twice the window's width and height before being smoothly downsampled. Avatar textures also use mipmaps and trilinear filtering. The render buffer follows window resizing; this avoids requesting a window MSAA mode that can fail on some GLX drivers.

The resizable studio opens at 1280 x 960 with a 960 x 720 minimum window size. Avatar size preserves aspect ratio and reserves enough headroom for maximum bounce. The full sidebar and mouse hit areas scale together with the window so the additional frequency controls remain easy to tune.

Linux needs ALSA + X11/GL dev packages (`libasound2-dev libx11-dev libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev libgl1-mesa-dev`) plus `cmake` and `clang`.

See [MANUAL.md](MANUAL.md) for shortcuts and troubleshooting. Run `cargo test -p mic_freq` to test voice states, motion, blinking, settings, CLI arguments, image discovery, layout, and the existing time-tool library.

## OBS setup

1. Select green or magenta, avoiding colors in your artwork.
2. Click **PNGTuber box**, or launch with `--capture`.
3. Add an OBS Window Capture source for **mic_freq - PNGTuber Studio**. On Wayland, use the capture method supported by your compositor and OBS, such as portal Screen Capture.
4. Add a **Chroma Key** filter matching the background; adjust similarity and spill reduction around the edges. Crop window decorations if your capture method includes them.
5. Press **C** in the app to restore its controls/frame, then return to the PNGTuber box before going live.

The window itself is opaque: image transparency is composited onto the selected background. This is a PNGTuber application, not a Live2D/VRM renderer or webcam face tracker. Hotkeys require the app window to be focused; global shortcuts and direct OBS integration are not implemented. GIFs are still images, not animated playback.

## Saved preferences

Gate, loud sensitivity, Low/Mid/High voice gains, frequency-specific animation controls, bounce, size, mirror, blink, background, input device, and avatar folder save automatically after edits and on normal exit. Pitch-specific images remain in the selected avatar folder. Linux uses `$XDG_CONFIG_HOME/mic_freq/settings.json`, or `~/.config/mic_freq/settings.json`. Mute, manual pose, and PNGTuber-box mode are session-only; `--capture` enables the box at startup. `--device` and a folder argument override saved choices and become the new preferences.

Existing alarm/timer/stopwatch and grid-launch source modules are preserved, but are not wired into this PNGTuber executable.
