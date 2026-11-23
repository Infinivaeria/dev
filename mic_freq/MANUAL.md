# Selenite Audio Studio manual

## Requirements

In the full Selenite extraction package, run `./mic_freq/mic_freq` directly from
its root. It resolves the package root, adjacent `mic_freq/assets`, portable
tools and preferences without a launcher script or working-directory setup.
Use `./mic_freq/mic_freq /path/to/avatar --capture` to override the artwork.
The legacy `run-mic-freq.sh` wrapper, when present, remains only a compatibility
shim.
Rust and development headers are only needed to build from source, not to run
the extracted executable. The host still supplies graphics, ALSA and microphone
permissions. The dedicated game-only package does not contain this studio.

- A microphone input device for voice animation. Manual poses and the demo avatar work without one.
- Rust nightly, `cmake`, `clang`; on Linux: `libasound2-dev libx11-dev libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev libgl1-mesa-dev`.

## Avatar images

Place `idle.png`, `talking.png`, and `loud.png` in your avatar folder. From the repository root, run `cargo run -p mic_freq --release -- /path/to/images`. An explicit folder argument overrides everything. Otherwise, an existing `assets/` folder beside the executable takes priority over the remembered folder; missing remembered folders are ignored. This works when you copy the executable and artwork elsewhere. The selected directory is printed as `Avatar images: ...` at startup. Relative arguments use the working directory.

Each image is a complete avatar pose, not a part to be overlaid. The program shows only one at a time, scaled to fit the window and preserving its aspect ratio. Transparent PNGs work well for avatar artwork. Supported extensions: png, jpg/jpeg, bmp, gif (first frame), qoi and tga. Names and extensions are case-insensitive. Keep only one matching image per state (e.g. do not have both `idle.png` and `idle.jpg`).

If a state has no image, an animated demo character appears. Press **R** to reload after editing. Drop one folder or pose image to switch avatar sets. Invalid or duplicate images produce an error; a failed reload keeps the old set intact.

Optional `idle_blink.png`, `talking_blink.png`, and `loud_blink.png` are full-frame closed-eye versions of their corresponding poses. Each blink image must have the same dimensions as its base pose. Blinks last about 220 ms, with intervals varying between 2 and 5 seconds. Missing blink images fall back to the base pose. Use identical canvas dimensions and character alignment across all poses to avoid visual jumps.

Click **Artwork** in Audio Studio to browse directories and replace any of the
six global open/closed pose slots. The same in-app browser is available in each
frequency window. Selecting a file copies it into the avatar folder with the
correct pose name; dropping a file remains supported.

Pitch-specific poses use a `low_`, `mid_`, or `high_` prefix:
`low_idle.png`, `mid_talking.png`, `high_loud.png`, and matching `_blink`
variants such as `low_talking_blink.png`. They accept every image format listed
above. Missing frequency poses use the global pose; missing closed-eye frequency
poses use that frequency's open pose.

## Controls

The title-row buttons open **Presets**, **Artwork**, and **Help**. F1 also opens
the in-app help tab, which summarizes this manual and the PNGTuber-only mode.
Music Composer and
Graphics Studio belong to the global Selenite toolbar and suite menu, keeping
Audio Studio focused on microphone-driven PNGTuber presentation.

| Key | Action |
|-----|--------|
| Up / Down | Raise / lower the noise gate by 2 dB |
| R | Reload avatar images |
| C | Toggle PNGTuber-only box / studio controls; click the avatar box to return |
| F1 | Open/close the in-app mic_freq Help tab |
| M | Mute microphone animation (forces idle) |
| 0 | Automatic microphone-driven pose |
| 1 / 2 / 3 | Force idle / talking / loud pose |
| B | Toggle bounce off / normal strength |
| X | Mirror avatar horizontally |
| Esc | Quit and save preferences |

Hotkeys work only while the app has keyboard focus. The mouse-accessible sidebar includes pose selection, gate/Low/Mid/High gain/loud-threshold/bounce/size sliders, mute/mirror/blink checkboxes, background swatches, device selection, calibration, Help, and PNGTuber-only capture. The studio opens at 1280 x 960 and scales its entire 340-logical-pixel sidebar and hit areas together down to its 960 x 720 minimum.

The default gate is -45 dBFS. Talking starts at gate + 4 dB and returns to idle below the gate. Loud talking requires 300 ms of sustained input above gate + 24 dB by default and returns to talking 4 dB below its entry threshold. A drop below the loud threshold resets the entry timer. Idle enters talking first, even for loud input. The loud-threshold slider adjusts that offset from 8 to 36 dB without changing the gate; raise it if normal speech reaches loud too easily. Volume smoothing reduces flicker. Mute overrides both microphone input and forced poses; it does not mute the system microphone or OBS audio.

Each state bounces continuously with smoothly changing strength. At 100%, idle rises up to 3 logical pixels, talking 16, and loud talking 30. Strength can be set from 0 to 200%. The avatar reserves bounce headroom, scales with the window, and preserves aspect ratio. Blink art switches without changing bounce phase.

The image choice depends on microphone loudness after the selected frequency response is applied. Low fundamental pitches below 160 Hz use **Low voice gain**, pitches from 160 through 300 Hz use **Mid voice gain**, and higher pitches use **High voice gain**. Each adjustment ranges from -18 to +18 dB; a missing or unpitched signal uses the raw level. This allows different sensitivity for deeper, mid-range, and brighter voices without recording or transcribing speech.

Click **Edit** beside a Low, Mid, or High gain slider to open a separate
frequency window. That window configures the band's gain, bounce, avatar size,
mirroring and blinking. Select Idle/Talking/Loud open or closed-eye artwork and
drop any supported image into that slot. The editor stores frequency artwork
beside the other avatar images and uses the same case-insensitive image names.
Custom animation starts off, preserving the global bounce, size, mirror and
blink settings. Turning it on (or changing one of its motion controls) enables
the Low/Mid/High-specific presentation override without changing pitch artwork.
Multiple frequency editor windows can remain open; closing one reloads Audio
Studio.

## Composer

Open **Composer** from Selenite or run `selenite --composer [PROJECT.json]`.
The window starts at 960 x 720 and scales the complete canvas to any larger
window; controls are never rendered at independent pixel sizes.

Frequency and Artwork option windows use the same approach: the complete 900 x
760 logical UI is proportionally fitted to the current window, including all
buttons, sliders, image slots and browser hit targets.

### Tracks and piano roll

The four tracks have independent oscillator/sample sources, envelope presets,
mute, volume and stereo pan. The piano roll displays C4 through B4 over sixteen
steps. Click a cell to toggle that pitch at that step and click a track to edit
it. Arrow keys move between tracks and steps.

Waveforms are sine, square, saw, triangle and deterministic noise. Envelopes are
Default, Pluck and Pad. To use imported audio as an instrument, select the track
and drop one WAV or OGG file anywhere in the Composer window.

### Theory tools

Cycle Root, Scale and Chord quality, then choose an inversion. **Add Chord**
writes the current chord to the selected step. **I-IV-V-I** writes four chords
at steps 1, 5, 9 and 13. **Arp** writes an eight-step up, down, up/down, down/up
or deterministic-random pattern from the selected step's notes. Analysis reports
the closest exact chord and the strongest major/natural-minor key for all notes
on the selected track.

### Project and media workflow

- **New** resets to a four-track project at 120 BPM.
- **Save** or Ctrl+S writes JSON under
  `$XDG_CONFIG_HOME/mic_freq/projects` unless a project path was loaded.
- **Export WAV** renders deterministic 44.1 kHz floating-point stereo PCM beside
  the project.
- Drop JSON to load a project.
- Drop standard `.mid` or `.midi` to import its tracks, tempo and note events.
- Drop WAV or OGG to assign a sampled instrument to the selected track.
- Cycle the **Device** button before Play when several output devices exist.
- Space starts/stops; Escape closes the Composer.

For headless verification, `mic_freq --self-test` validates project JSON, theory,
deterministic synthesis, A4 pitch detection, rhythm tracking and WAV round-trip
without requiring a microphone, output device or display.

Playback uses the selected output device explicitly. If no device is available
or project/sample data is invalid, the status bar reports the host or format
error without changing the current project.

## Signal analyzer

The **Analyzer** button shares the clean-capture row and toggles a live panel
over the avatar canvas. It reports:

- level in dBFS;
- fundamental pitch in hertz, nearest note and cents from concert A 440 Hz;
- harmonicity and eight relative harmonic bars;
- tonal and percussive balance, brightness and confidence;
- onset/transient strength, pulse regularity and estimated BPM.

Pitch and harmonic measurements update several times per second from the latest
microphone window. Rhythm uses a sample sequence number to process only new
frames, not overlapping display windows. Percussive content naturally has weaker
single-note pitch confidence than a sustained voice or oscillator.

Click **Analyze Mix** in Composer to run the same analysis on a downmixed
44.1 kHz render. A final mix may not contain one stable pitch or pulse, and the
UI displays an em dash instead of a false result.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| No input / disconnected input | Click **Next input / reconnect**, set a system default, or use `--list-devices` and `--device "NAME"`. |
| Always idle | Lower the gate with **Down** or raise microphone gain. |
| Always talking | Raise the gate with **Up** to filter background noise. |
| Demo character instead of artwork | Check the selected folder and filenames, then press **R**. |
| Reload error | Check image format and remove duplicate state images; the terminal contains the full error. |
| Loud pose never appears | Lower the loud threshold, or increase microphone gain. |
| No blinking | Add matching `_blink` images and enable **Blink**. |
| Background visible in OBS | Enable PNGTuber box and apply an OBS Chroma Key filter. |
| Artwork disappears with chroma key | Choose a background color absent from the artwork. |
| Hotkeys do nothing while gaming | Focus mic_freq; global hotkeys are not implemented. |
| Preferences do not save | Check write permissions on the configuration directory. Save errors appear in the studio or terminal. |
| Composer has no output | Cycle **Device** and verify the host exposes an enabled audio output. |
| Imported sound is silent | Use a valid WAV/OGG file and add notes to the selected track. |
| MIDI has no notes | Verify the file contains Note On events rather than only automation or percussion metadata. |
| Pitch reads no value | Sustain a pitched sound above the noise gate; noise and drums may not have a fundamental. |
| BPM is unstable | Feed at least several regular beats; tempo needs multiple distinct onsets. |

## Calibration and capture

Click **Calibrate noise gate** and remain quiet for three seconds. The gate is set 6 dB above the highest measured room-noise level, within the supported range. Avoid tapping the microphone during calibration. Then speak normally and fine-tune the gate and loud threshold. Calibration requires a working microphone and is not saved until it finishes.

See [OBS setup](README.md#obs-setup) for capture and chroma-key steps. PNGTuber box hides all in-app UI, errors, and the operating-system window frame, including microphone and reload failures. Return to the studio with **C** or by clicking the avatar box once to diagnose a problem. The window has no native transparent-background mode; PNG alpha is composited onto the chosen background.

## Preferences and CLI

Run `cargo run -p mic_freq -- --help` for launch options. A folder argument or `--device NAME` overrides and updates saved preferences. `--list-devices` prints the available input names without opening a window. `--capture` starts without controls.
`--version` prints the installed `mic_freq` package version without opening a
window or microphone.

Preferences, including Low/Mid/High voice gains, save after changes and on normal exit using an atomic file replacement. Linux stores them at `$XDG_CONFIG_HOME/mic_freq/settings.json`, defaulting to `~/.config/mic_freq/settings.json`. Delete that file while the app is closed to reset preferences. Mute, forced poses, and capture mode are not restored on startup.

The same preferences file also stores independent Low/Mid/High animation
settings. Use `--frequency low`, `--frequency mid`, or `--frequency high` to
open an editor directly, optionally followed by the avatar folder.

This executable focuses on PNGTuber streaming. Existing time-tool and grid-launch source modules remain in the repository but are not connected to its UI. Webcam face tracking, Live2D/VRM models, animated GIF playback, global hotkeys, and direct OBS control are outside its current capabilities.
