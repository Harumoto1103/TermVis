# TermVis

Watch video streams and monitor CV models directly over SSH — no X11, no GUI required.

Built with Rust + OpenCV. Renders frames as high-fidelity half-block characters with lossless incremental compression and Fourier sharpening.

---

## Why

When you're working on a remote GPU server via SSH, you have no way to see what your vision model is actually looking at. Downloading frames one by one is slow, and streaming high-res video over a laggy connection is painful.

TermVis addresses this by:

1. Converting video frames to half-block (`▀`) characters with 24-bit color, effectively doubling terminal vertical resolution.
2. Compressing only the *diff* between frames using a Delta-XOR + Zlib scheme (`.lzdx` format), so static backgrounds cost almost nothing.
3. Applying DFT (Fourier) sharpening on the fly to recover edge detail at terminal resolutions.

## Install

```bash
pip install termvis
```

## Quick Start

```python
import termvis

# Preview a webcam or video file
termvis.quick_play(0)          # camera index
termvis.quick_play("clip.mp4") # or a file path
```

Press `q` to quit.

## API

### `TermVis` class

```python
from termvis import TermVis
```

Use as a context manager — it hides the cursor and switches to the alternate screen buffer on enter, and restores terminal state on exit.

```python
with TermVis() as tv:
    ...
```

| Method | Description |
|--------|-------------|
| `render(frame_bgr)` | Render an OpenCV BGR frame to the terminal. Handles color conversion and adaptive resizing. |
| `poll_key()` | Non-blocking key read. Returns a string like `'q'`, `'esc'`, `'enter'`, or `None`. |
| `start_recording(path)` | Start saving frames to a `.lzdx` file. |
| `stop_recording()` | Flush and close the current recording. |
| `play_recorded(path, sharpen=0.3)` | Play back a `.lzdx` recording. `sharpen` controls DFT filter strength (0.0–1.5). |
| `get_mapping_info()` | Returns a dict with terminal dimensions, render height, and original frame size. |
| `map_coords(col, row)` | Translate 1-based terminal character coordinates to original frame pixel coordinates. |

### `quick_play(source=0)`

High-level helper for camera or file preview. `q` to quit.

## Examples

See the [`examples/`](examples/) directory:

| File | What it shows |
|------|---------------|
| `basic_demo.py` | Minimal render loop |
| `interactive_painter.py` | Draw on a high-res canvas using terminal mouse clicks |
| `remote_control_demo.py` | Mouse event parsing and coordinate mapping |
| `remote_desktop.py` | Mirror a physical display into SSH with mouse passthrough |

## How it works

**Rendering** — Each terminal cell holds two pixels stacked vertically via the `▀` half-block character, with independent 24-bit foreground/background colors. This doubles effective vertical resolution.

**Compression** — Consecutive frames are XOR'd and the diff is zlib-compressed. Static regions compress to near-zero. The result is stored as `.lzdx` (Delta-XOR).

**Engine** — Pixel sampling and byte manipulation run in a Rust core via PyO3, keeping the hot path off the Python GIL.

## License

MIT
