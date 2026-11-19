# Selenite Graphics Studio

Selenite Graphics Studio is the native image, animation and video utility in the
16.0 release line. Open it from the organizer toolbar, **Suite > Graphics
Studio**, or directly:

```sh
./selenite --graphics-studio
./selenite --graphics-studio image.png
```

Audio Studio's **Graphics** button opens the adjacent Selenite executable, so a
PNGTuber workflow can move between pose creation and microphone animation
without a shell launcher.

## Images and layers

Drop one or more images into the workspace. PNG, JPEG, WebP, BMP, TIFF, QOI and
GIF are decoded by the Rust `image` crate. The layer sidebar supports selection,
visibility, opacity cycling, reordering and removal. The preview is fitted and
scaled without stretching.

Each edit writes an atomic `-edited.png` beside the original; originals are not
overwritten. Available operations include center crop, half-size resize, 90-degree
rotation, horizontal/vertical flip, grayscale, inversion, brightness, contrast
and imageproc Gaussian blur. Undo/redo changes the selected working image without
rewriting the original.

Graphics projects save image paths, canvas size, visibility, opacity and offsets
as JSON. **Export PNG** flattens visible layers with alpha compositing.

## PNGTuber artwork

Select an image and click **PNGTuber**. Graphics Studio creates:

- `idle.png` and `idle_blink.png`
- `talking.png` and `talking_blink.png`
- `loud.png` and `loud_blink.png`

The files are written under `pngtuber-export` beside the source. Two conservative
blink bars are generated from the image geometry and the original output remains
untouched. Select that folder as Audio Studio's avatar assets, or replace the
generated poses with custom artwork.

The same operation is available headlessly:

```sh
./selenite --graphics-pngtuber avatar.png exported-poses
./selenite --graphics-export project.json flattened.png
```

## Animation and video

Animated GIFs show frame count/delay, previous/play/next controls, and use the
same scaled preview. Static GIF files remain ordinary image layers.

When Selenite is built with the `native-video` feature, probing, decoding and
H.264 encoding use `video-rs`/FFmpeg libraries in-process. Other builds use the
portable `ffprobe` and `ffmpeg` executables bundled under `ffmpeg/bin`; adjacent
tools are preferred before PATH. Commands are launched directly, never through a
shell, and missing tools are reported in the status bar:

```sh
./selenite --graphics-extract input.mp4 frames 12
./selenite --graphics-transcode input.mov output.mp4 \
  -c:v libx264 -crf 23 -pix_fmt yuv420p -c:a aac
```

`package.sh` bundles the ffmpeg/ffprobe executables and the required libav/libsw
runtime libraries into `dist/ffmpeg/bin` and `dist/lib`. Set
`SELENITE_FFMPEG_BIN`, `SELENITE_FFPROBE_BIN`, and (optionally)
`SELENITE_NATIVE_VIDEO=1` for a portable native-video build. `distribute.sh`
requires those media tools and validates the complete package before replacing
an installation.

Transcode options are allow-listed and argument pairs are validated. Arbitrary
input/output redirection, shell commands and unrecognized ffmpeg options are
rejected.

## Headless release checks

`mic_freq --self-test` verifies the connected Audio Studio/Graphics release
workflow without opening a display. The full `tests/graphics.rs` suite covers
layer compositing, filters, projects, GIF round-trip, PNGTuber files, ffmpeg
argument safety and the mouse-driven Graphics Studio workflow.
