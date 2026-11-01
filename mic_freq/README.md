# mic_freq

An offline PNGTuber studio built with **raylib** and **cpal**. Microphone volume selects idle, talking, or loud talking artwork. Each pose bounces smoothly: gentle idle breathing, talking hops, and stronger loud-talking hops. No speech recognition, recording, internet connection, or frequency matching is involved.

## Run

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

PNG, JPG/JPEG, BMP, GIF (first frame), QOI, and TGA are supported. Use transparent PNGs and identically sized canvases for consistent framing. Optional `idle_blink.png`, `talking_blink.png`, and `loud_blink.png` provide closed-eye variants; each must match its base pose's dimensions. Missing blink art falls back to the base pose. Only one image per name is allowed, case-insensitively.

## Studio controls

- Sliders adjust the noise gate, bounce strength (0-200%), avatar size, and loud threshold. The input meter shows microphone level in dBFS.
- **Calibrate noise gate** measures three seconds of room noise. Stay quiet during calibration, then test normal speech and adjust as needed.
- Checkboxes toggle microphone mute, horizontal mirroring, and automatic blinking.
- **Auto / Idle / Talk / Loud** select microphone control or a manual pose. Mute always forces idle.
- **Next input / reconnect** cycles available microphones and the system default. Capture failure does not close the app; manual poses still work.
- Drop one avatar folder or pose image into the window to switch sets. **R** reloads edited artwork. Failed reloads preserve the current set.
- Background swatches select green, magenta, or dark. **Clean capture** hides every control, meter, and error message. Press **C** to return.

The default gate is -45 dBFS; talking starts 4 dB above it. Loud talking starts 24 dB above it by default, independently adjustable from 8 to 36 dB above the gate. It requires 300 ms of sustained loud input while talking; brief peaks do not trigger it, and idle never jumps directly to loud. Lower the loud threshold if your microphone never reaches the loud pose, or raise it if normal speech triggers loud too easily. Smoothing and 4 dB hysteresis reduce flicker.

Full-scene anti-aliasing uses 4-sample supersampling: the avatar and text render at twice the window's width and height before being smoothly downsampled. Avatar textures also use mipmaps and trilinear filtering. The render buffer follows window resizing; this avoids requesting a window MSAA mode that can fail on some GLX drivers.

The resizable studio has a 720 x 600 minimum window size. Avatar size preserves aspect ratio and reserves enough headroom for maximum bounce. Controls and mouse hit areas scale together.

Linux needs ALSA + X11/GL dev packages (`libasound2-dev libx11-dev libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev libgl1-mesa-dev`) plus `cmake` and `clang`.

See [MANUAL.md](MANUAL.md) for shortcuts and troubleshooting. Run `cargo test -p mic_freq` to test voice states, motion, blinking, settings, CLI arguments, image discovery, layout, and the existing time-tool library.

## OBS setup

1. Select green or magenta, avoiding colors in your artwork.
2. Click **Clean capture**, or launch with `--capture`.
3. Add an OBS Window Capture source for **mic_freq - PNGTuber Studio**. On Wayland, use the capture method supported by your compositor and OBS, such as portal Screen Capture.
4. Add a **Chroma Key** filter matching the background; adjust similarity and spill reduction around the edges. Crop window decorations if your capture method includes them.
5. Press **C** in the app to edit settings, then return to clean capture before going live.

The window itself is opaque: image transparency is composited onto the selected background. This is a PNGTuber application, not a Live2D/VRM renderer or webcam face tracker. Hotkeys require the app window to be focused; global shortcuts and direct OBS integration are not implemented. GIFs are still images, not animated playback.

## Saved preferences

Gate, loud sensitivity, bounce, size, mirror, blink, background, input device, and avatar folder save automatically after edits and on normal exit. Linux uses `$XDG_CONFIG_HOME/mic_freq/settings.json`, or `~/.config/mic_freq/settings.json`. Mute, manual pose, and clean-capture mode are session-only; `--capture` enables clean capture at startup. `--device` and a folder argument override saved choices and become the new preferences.

Existing alarm/timer/stopwatch and grid-launch source modules are preserved, but are not wired into this PNGTuber executable.
