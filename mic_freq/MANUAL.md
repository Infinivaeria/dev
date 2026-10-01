# mic_freq — User Manual

mic_freq listens to your microphone, finds the strongest (dominant) frequency in the sound, and animates a row of images. Each image responds to its own frequency band, so low sounds light up the images on the left and high sounds light up the images on the right.

---

## 1. Requirements

- A working microphone set as the system's **default input device**.
- Rust toolchain (1.88+), `cmake` and `clang`.
- Linux: `libasound2-dev libx11-dev libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev libgl1-mesa-dev`.

## 2. Starting the program

```sh
cd mic_freq
cargo run --release                     # loads images from ./assets
cargo run --release -- /path/to/images  # loads images from another folder
```

The image folder path is relative to the **current working directory**, so run the default command from inside `mic_freq/`.

On startup the terminal prints the microphone in use, for example:

```
Listening on 'default' @ 44100 Hz
```

Close the window or press **Esc** to quit. If there are unsaved image edits, you are asked to **Save**, **Discard** or **Cancel** first.

## 3. Controls

| Key   | Action                                                          |
|-------|-----------------------------------------------------------------|
| Up    | Raise the noise gate by 2 dB (less sensitive; max −6 dB)        |
| Down  | Lower the noise gate by 2 dB (more sensitive; min −90 dB)       |
| Tab   | Open / close the image editor panel (see §6)                    |
| Ctrl+S | Save image settings to `mic_freq.json`                         |
| Esc   | Close the editor if it is open, otherwise quit                  |
| Left click on an image | Select it in the editor (opens the panel)       |
| Drag & drop files | Add images (or replace the selected one, see §6.3)   |

Keyboard shortcuts are ignored while you are typing in the editor's **Name** box. Press Enter or click elsewhere to finish typing.

The window can be resized; the layout adapts.

## 4. Reading the screen

```
┌───────────────────────────────────────────────────────────────┐
│  440.0 Hz   A4 +0c                                     60 FPS │  ← header
│  input  -23.4 dBFS   gate -50 dB (Up/Down)                    │
│                                                               │
│   [img]     [img]     [IMG]     [img]     [img]               │  ← images
│  bass 100Hz L0  ...  mid 400Hz L5  ...                        │  ← labels
│   ▪▪▪▪▪     ▪▪▪▪▪     ▪▪▪▪▪     ▪▪▪▪▪     ▪▪▪▪▪               │  ← level bars
│ ┌───────────────────────────────────────────────────────────┐ │
│ │  spectrum  │      │    ▌█▌   │      │                     │ │  ← spectrum
│ └───────────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────┘
```

**Header**
- **Frequency**: the dominant frequency in Hz. It shows `--- Hz (below gate)` when the input is too quiet.
- **Note**: the nearest musical note (A4 = 440 Hz) and how far off it is in cents. `+12c` means 12 cents sharp.
- **input dBFS**: the current microphone loudness. 0 dBFS is the loudest possible.
- **gate**: the current noise gate threshold.

**Images**: each image grows, brightens, rises and wobbles as its level increases.

**Labels**: `name  center-frequency  L<step>`, where the step goes from L0 (idle) to L5 (maximum).

**Level bars**: five blocks per image that fill from green to red as the level goes from L1 to L5.

**Spectrum**: a live frequency spectrum from 30 Hz to 4 kHz on a logarithmic axis.
- Blue vertical lines mark each image's center frequency.
- The yellow line marks the detected dominant frequency.
- With the editor open, the selected image's band is shaded yellow and the image itself has a yellow outline.

## 5. The images

### 5.1 Built-in placeholder images

If the image folder is missing, empty, or has no supported images, five images are generated automatically:

| # | Name       | Center frequency | Appearance                          |
|---|------------|------------------|-------------------------------------|
| 1 | `bass`     | 100 Hz           | Red radial glow                     |
| 2 | `low-mid`  | 200 Hz           | Orange / brown checkerboard         |
| 3 | `mid`      | 400 Hz           | Grey cellular (Voronoi) pattern     |
| 4 | `high-mid` | 800 Hz           | Grey Perlin noise                   |
| 5 | `treble`   | 1600 Hz          | Light blue → purple radial glow     |

The terminal prints `No images found in '...', using generated placeholder assets` when these are used.

### 5.2 Using your own images

1. Put image files in `mic_freq/assets/` or in another folder that you pass on the command line.
2. Supported formats: **png, jpg/jpeg, bmp, gif** (first frame only), **qoi, tga**. Extensions are not case-sensitive.
3. **Each image is named after its file name without the extension.** For example, `kick.png` is shown as `kick`.
4. **Initial order = alphabetical by file name.** This applies only to files not yet in `mic_freq.json`. Once you save from the editor, the saved order, names and frequencies win (§6.4). The first file gets the lowest frequency band and the last file gets the highest. To control the order, prefix file names with numbers:

   ```
   assets/
     1_drum.png     → lowest band
     2_guitar.png
     3_voice.png
     4_whistle.png  → highest band
   ```

5. Files that can't be loaded are skipped with a `skipping <file>: <error>` message.
6. Any number of images works. New images (with no saved settings) are spread evenly on a musical (log) scale from **100 Hz to 1600 Hz**:

| Images | Center frequencies (Hz)                 |
|--------|-----------------------------------------|
| 1      | 400                                     |
| 2      | 100, 1600                               |
| 3      | 100, 400, 1600                          |
| 4      | 100, 252, 635, 1600                     |
| 5      | 100, 200, 400, 800, 1600                |
| 6      | 100, 174, 303, 528, 919, 1600           |

## 6. Customizing images (editor panel)

Press **Tab** (or click any image) to open the editor on the right side of the window. Everything you change is applied live; nothing is written to disk until you **Save**.

```
┌ Image editor *  [Tab] hide ──────┐   * = unsaved changes
│ Images (5) - left = low ...      │
│ ┌──────────────────────────────┐ │
│ │ bass  (100 Hz)               │ │   ← image list (click to select)
│ │ mid   (400 Hz)   ◄ selected  │ │
│ └──────────────────────────────┘ │
│ [< Left][Right >][Remove][Spread]│
│ ☐ Dropped file replaces selected │
│ Source / Name / Visible          │
│ Frequency, Band width, ... sliders│
│ Tint colour picker               │
│ [Reset tint][Reset all settings] │
│ [Save (Ctrl+S)][Revert to saved] │
└──────────────────────────────────┘
```

### 6.1 Image list and buttons

| Control | What it does |
|---------|--------------|
| List | Click an image to select it. You can also click the image on the stage. |
| **< Left** / **Right >** | Move the selected image one slot left or right on the stage. This only changes display order, not its frequency. |
| **Remove** | Take the image off the stage. The file is **not** deleted; it is added to the `ignored` list so it doesn't come back on the next start. Dropping the same file again brings it back. |
| **Spread** | Re-assign every image's frequency so they are evenly log-spaced from 100 to 1600 Hz in their current left-to-right order. |
| **Dropped file replaces selected image** | When ticked, a dropped file swaps the selected image's picture but keeps all its settings. |

### 6.2 Properties of the selected image

| Property | Range | Meaning |
|----------|-------|---------|
| Source | – | File name inside the image folder, or `built-in placeholder`. |
| Name | up to 40 chars | Label under the image. Click the box, type, then press Enter. |
| Visible | on/off | Hidden images are not drawn and take no slot on the stage. They still appear in the list. |
| Frequency | 50 – 4000 Hz (log slider) | Band center: the pitch that drives this image hardest. **Use current pitch** sets it to whatever you are singing or playing right now (greyed out while the input is below the gate). |
| Band width | 0.1 – 2 octaves | How far from the center the image still reacts. Wider = more overlap with neighbours. |
| Sensitivity | ×0 – ×4 | Multiplies loudness, so quiet sounds reach full level sooner. |
| Size | ×0.1 – ×3 | Overall size relative to the slot. |
| Idle size | 0 – 100 % | Size at level 0, as a share of full size. |
| Idle opacity | 0 – 100 % | Opacity at level 0. |
| Lift | 0 – 200 px | How far the image rises at full level. |
| Wobble | 0 – 45° | Maximum rocking angle at full level. |
| Tint | RGB colour | Multiplied with the picture. White = original colours. **Reset tint** sets it back to white. |

**Reset all settings** puts every other property of the selected image back to its default (including Visible). The name, source and frequency are kept.

### 6.3 Adding and replacing images

Drag one or more image files from your file manager onto the window:

- Files from elsewhere are **copied** into the image folder. If a file with that name already exists, a number is appended (`kick-2.png`).
- A new image's frequency is the current detected pitch. If the input is silent, it gets the last image's frequency × √2 (or 400 Hz if there are none).
- With **Dropped file replaces selected image** ticked, the first dropped file replaces the selected image's picture instead.
- Unsupported files are ignored. The status line at the bottom of the panel says what happened.

### 6.4 Saving: `mic_freq.json`

**Save** (or Ctrl+S) writes `mic_freq.json` into the image folder. **Revert to saved** discards all changes since the last save. Example:

```json
{
  "images": [
    {
      "name": "Kick drum",
      "source": "1_drum.png",
      "visible": true,
      "center_hz": 100.0,
      "width_octaves": 0.6,
      "gain": 1.0,
      "scale": 1.0,
      "min_size": 0.45,
      "min_opacity": 0.25,
      "lift_px": 60.0,
      "wobble_deg": 4.0,
      "tint": [255, 200, 0]
    }
  ],
  "ignored": []
}
```

- The **array order is the stage order** (left → right).
- `source` is relative to the image folder. `generated:<name>` means a built-in placeholder.
- On start, entries whose file no longer exists are dropped. Image files not listed (and not in `ignored`) are appended with fresh frequencies.
- Missing fields get their default values and out-of-range values are clamped, so you can safely hand-edit the file.
- The save is atomic: a temporary file is written and then renamed, so a crash can't leave a half-written file.

## 7. How an image's level is computed

1. **Detection**: about 93 ms of audio (4096 samples at 44.1 kHz) is analysed each frame. The strongest peak between **50 and 4000 Hz** becomes the dominant frequency.
2. **Gate**: if the input is quieter than the gate, no frequency is reported and all images fade out.
3. **Band match**: each image scores 1.0 when the frequency is exactly at its center. The score falls off smoothly (Gaussian). With the default 0.6-octave **Band width**, it is about 0.25 one octave away.
4. **Loudness**: the score is multiplied by loudness × **Sensitivity** (capped at 1). Loudness is 0 at the gate and 1 at 35 dB above the gate.
5. **Smoothing**: levels rise quickly (attack) and fall slowly (release), so the visuals don't flicker.
6. **Visual mapping** for level 0 → 1 (defaults; all adjustable per image in the editor):
   - size: Idle size (45 %) → 100 %, times Size
   - opacity: Idle opacity (25 %) → 100 %
   - lift: 0 → Lift (60 px)
   - wobble: 0 → ±Wobble (4°)
   - step: L0 → L5

## 8. Tuning

Per-image settings are best changed in the editor (§6). The global constants are at the top of `src/main.rs`:

| Constant             | Default | Effect                                              |
|----------------------|---------|-----------------------------------------------------|
| `LOUDNESS_RANGE_DB`  | 35.0    | dB above the gate needed to reach full level        |
| `LEVEL_STEPS`        | 5       | Number of discrete steps (L0..Ln)                   |
| `ATTACK_PER_SEC`     | 14.0    | How fast levels rise                                |
| `RELEASE_PER_SEC`    | 3.0     | How fast levels fall                                |
| `FFT_SIZE`           | 4096    | Analysis window; larger = finer but slower response |
| `SPECTRUM_MAX_HZ`    | 4000.0  | Right edge of the spectrum display                  |

In `src/config.rs`: `DEFAULT_LO_HZ`/`DEFAULT_HI_HZ` (100/1600 Hz, the range used for automatic spreading) and `MIN_HZ`/`MAX_HZ` (50/4000 Hz, the detection range and Frequency slider limits). The default gate (−50 dB) is on `Analyzer` in `src/analysis.rs`.

## 9. Troubleshooting

| Symptom                                    | Fix                                                                 |
|--------------------------------------------|---------------------------------------------------------------------|
| `no default input (microphone) device found` | Plug in a mic or set a default input in your system sound settings. |
| Always `below gate`                        | Press **Down** to lower the gate, or raise the mic gain.            |
| Jumps around in a quiet room               | Press **Up** to raise the gate above the background noise.          |
| Placeholders appear instead of my images   | Check the folder path and working directory, and use a supported format. |
| Only the edge images react                 | Your sounds fall outside the bands. Sing/play a note, select an image and press **Use current pitch**. |
| Voice detected an octave too high          | Harmonics can be stronger than the fundamental; increase **Band width**. |
| A removed image keeps coming back          | Save after removing it. Removal is only remembered in `mic_freq.json`. |
| My edits were lost                         | Edits are kept in memory until **Save** / Ctrl+S. Check the `*` in the panel title. |
| Want to start over                         | Delete `mic_freq.json` from the image folder. |
| Keys (Up/Down/Tab) do nothing              | You are typing in the Name box; press Enter or click elsewhere. |
