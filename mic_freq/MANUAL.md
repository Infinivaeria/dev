# mic_freq — Voice avatar manual

## Requirements

- A microphone input device for voice animation. Manual poses and the demo avatar work without one.
- Rust nightly, `cmake`, `clang`; on Linux: `libasound2-dev libx11-dev libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev libgl1-mesa-dev`.

## Avatar images

Place `idle.png`, `talking.png`, and `loud.png` in your avatar folder. From the repository root, run `cargo run -p mic_freq --release -- /path/to/images`. An explicit folder argument overrides everything. Otherwise, an existing `assets/` folder beside the executable takes priority over the remembered folder; missing remembered folders are ignored. This works when you copy the executable and artwork elsewhere. The selected directory is printed as `Avatar images: ...` at startup. Relative arguments use the working directory.

Each image is a complete avatar pose, not a part to be overlaid. The program shows only one at a time, scaled to fit the window and preserving its aspect ratio. Transparent PNGs work well for avatar artwork. Supported extensions: png, jpg/jpeg, bmp, gif (first frame), qoi and tga. Names and extensions are case-insensitive. Keep only one matching image per state (e.g. do not have both `idle.png` and `idle.jpg`).

If a state has no image, an animated demo character appears. Press **R** to reload after editing. Drop one folder or pose image to switch avatar sets. Invalid or duplicate images produce an error; a failed reload keeps the old set intact.

Optional `idle_blink.png`, `talking_blink.png`, and `loud_blink.png` are full-frame closed-eye versions of their corresponding poses. Each blink image must have the same dimensions as its base pose. Blinks last about 140 ms, with intervals varying between 2.5 and 5.5 seconds. Missing blink images fall back to the base pose. Use identical canvas dimensions and character alignment across all poses to avoid visual jumps.

## Controls

| Key | Action |
|-----|--------|
| Up / Down | Raise / lower the noise gate by 2 dB |
| R | Reload avatar images |
| C | Toggle clean capture / studio controls |
| M | Mute microphone animation (forces idle) |
| 0 | Automatic microphone-driven pose |
| 1 / 2 / 3 | Force idle / talking / loud pose |
| B | Toggle bounce off / normal strength |
| X | Mirror avatar horizontally |
| Esc | Quit and save preferences |

Hotkeys work only while the app has keyboard focus. The mouse-accessible sidebar includes pose selection, gate/bounce/size/loud-threshold sliders, mute/mirror/blink checkboxes, background swatches, device selection, calibration, and clean capture.

The default gate is -45 dBFS. Talking starts at gate + 4 dB and returns to idle below the gate. Loud talking requires 300 ms of sustained input above gate + 24 dB by default and returns to talking 4 dB below its entry threshold. A drop below the loud threshold resets the entry timer. Idle enters talking first, even for loud input. The loud-threshold slider adjusts that offset from 8 to 36 dB without changing the gate; raise it if normal speech reaches loud too easily. Volume smoothing reduces flicker. Mute overrides both microphone input and forced poses; it does not mute the system microphone or OBS audio.

Each state bounces continuously with smoothly changing strength. At 100%, idle rises up to 3 logical pixels, talking 16, and loud talking 30. Strength can be set from 0 to 200%. The avatar reserves bounce headroom, scales with the window, and preserves aspect ratio. Blink art switches without changing bounce phase.

The image choice depends on microphone *loudness*, not the words you say or the pitch of your voice. Speech is not recorded or transcribed.

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
| Background visible in OBS | Enable clean capture and apply an OBS Chroma Key filter. |
| Artwork disappears with chroma key | Choose a background color absent from the artwork. |
| Hotkeys do nothing while gaming | Focus mic_freq; global hotkeys are not implemented. |
| Preferences do not save | Check write permissions on the configuration directory. Save errors appear in the studio or terminal. |

## Calibration and capture

Click **Calibrate noise gate** and remain quiet for three seconds. The gate is set 6 dB above the highest measured room-noise level, within the supported range. Avoid tapping the microphone during calibration. Then speak normally and fine-tune the gate and loud threshold. Calibration requires a working microphone and is not saved until it finishes.

See [OBS setup](README.md#obs-setup) for capture and chroma-key steps. Clean capture hides all UI and errors, including microphone and reload failures. Return to the studio with **C** to diagnose a problem. The window has no native transparent-background mode; PNG alpha is composited onto the chosen background.

## Preferences and CLI

Run `cargo run -p mic_freq -- --help` for launch options. A folder argument or `--device NAME` overrides and updates saved preferences. `--list-devices` prints the available input names without opening a window. `--capture` starts without controls.

Preferences save after changes and on normal exit using an atomic file replacement. Linux stores them at `$XDG_CONFIG_HOME/mic_freq/settings.json`, defaulting to `~/.config/mic_freq/settings.json`. Delete that file while the app is closed to reset preferences. Mute, forced poses, and capture mode are not restored on startup.

This executable focuses on PNGTuber streaming. Existing time-tool and grid-launch source modules remain in the repository but are not connected to its UI. Webcam face tracking, Live2D/VRM models, animated GIF playback, global hotkeys, and direct OBS control are outside its current capabilities.
