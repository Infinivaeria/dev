# mic_freq scripting examples

Run an example from the Selenite package root:

```sh
./mic_freq/mic_freq --check-script examples/mic_freq/<name>.rhai
./mic_freq/mic_freq --script examples/mic_freq/<name>.rhai
```

Scripts execute `update(input)` once per rendered frame. They receive:

- `frame`, `time`, and `dt`
- `db` and `adjusted_db`
- `pitch_hz` (`()` when no stable pitch exists)
- `band`, `pose`, and `muted`
- `gate_db` and `loud_delta_db`

All examples return only validated, temporary presentation overrides. They do
not access files, devices, network services, processes, or saved settings.

| Example | Purpose |
|---|---|
| `minimal.rhai` | Smallest valid script |
| `auto-controls.rhai` | Explicitly defer pose and band selection |
| `adaptive-bounce.rhai` | Scale bounce from microphone level |
| `quiet-idle.rhai` | Force idle below a chosen level |
| `loud-ceiling.rhai` | Jump to loud at very high input |
| `talking-floor.rhai` | Keep normal audible input in talking pose |
| `low-band-only.rhai` | Keep deep voices on low artwork |
| `mid-band-only.rhai` | Keep ordinary voices on mid artwork |
| `high-band-only.rhai` | Keep bright voices on high artwork |
| `pitch-ranges.rhai` | Custom 120/260 Hz band boundaries |
| `pitch-bands-wide.rhai` | Custom 90/500 Hz band boundaries |
| `boost-low-voice.rhai` | Temporary low-band gain boost |
| `reduce-high-voice.rhai` | Temporary high-band gain reduction |
| `quiet-room-gate.rhai` | Gate/loud preset for a quiet room |
| `noisy-room-gate.rhai` | Gate/loud preset for background noise |
| `streaming-style.rhai` | Larger, brighter presentation override |
| `minimal-motion.rhai` | Low motion and fixed scale |
| `dance-clock.rhai` | Deterministic time-based bounce pulse |
| `pose-sequence.rhai` | Cycle manual poses every two seconds |
| `mirror-clock.rhai` | Alternate mirroring every four seconds |
| `blink-speech.rhai` | Blink only during talking or loud poses |
| `freeze-on-mute.rhai` | Stable muted pose presentation |
| `loud-scale.rhai` | Grow the avatar as the signal strengthens |
| `pitch-size.rhai` | Scale artwork by pitch band |
| `band-mirror.rhai` | Mirror one or more pitch bands |
| `phase-reset.rhai` | Frame-safe periodic pulse with no random state |
| `sine-bounce.rhai` | Oscillating bounce derived from elapsed time |
| `weekday-demo.rhai` | Demonstrates arithmetic and branch-heavy computation |
| `loop-demo.rhai` | Demonstrates bounded loop-based computation |
| `function-demo.rhai` | Demonstrates reusable Rhai functions |

