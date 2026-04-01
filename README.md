# TermVis 🚀

Watch video streams and monitor your CV models directly over SSH.

High-performance terminal rendering with lossless incremental compression and frequency-domain sharpening. Built with Rust and OpenCV.

---

### Why?
You're working on a remote GPU server via SSH. You have no X11 forwarding, no GUI, and no way to see what your computer vision model is actually looking at. Downloading frames one by one is slow, and streaming high-res video is impossible over a laggy connection.

**TermVis** solves this by:
1. Converting video frames into high-fidelity ASCII/Half-block characters.
2. Using a custom **LZDX** format (Delta-XOR + Zlib) to send only the changes between frames.
3. Applying **DFT (Fourier) sharpening** on the fly so you can actually see details in a 80x24 terminal.

### Quick Start

```bash
# Get the core engine
pip install termvis

# Run a quick camera test
python -c "import termvis; termvis.quick_play(0)"
```

### API Reference 📚

#### `TermVis` Class
The main class for handling rendering and recording.

- **`__enter__()` / `__exit__()`**
  Context manager support. Automatically hides the cursor and enables the alternate buffer on entry, and restores terminal state on exit.
  
- **`render(frame_bgr: numpy.ndarray)`**
  Renders an OpenCV-style BGR frame to the terminal. It handles color conversion and adaptive resizing automatically.

- **`poll_key() -> str | None`**
  Checks for keyboard input without blocking. Returns the key string (e.g., `'q'`, `'esc'`, `'enter'`) or `None` if no key was pressed.

- **`start_recording(path: str)`**
  Initializes a recording session. All subsequent `render()` calls will be saved to the specified `.lzdx` file using incremental compression.

- **`stop_recording()`**
  Ends the current recording session and flushes the file to disk.

- **`play_recorded(path: str, sharpen: float = 0.3)`**
  Plays back a `.lzdx` file.
  - `path`: Path to the recording.
  - `sharpen`: Strength of the DFT sharpening filter (recommended: `0.0` to `1.5`).

- **`get_mapping_info() -> dict`**
  Returns a dictionary containing terminal dimensions, rendering height, and original frame size. Useful for custom coordinate calculations.

- **`map_coords(terminal_col: int, terminal_row: int) -> (int, int)`**
  Translates terminal character coordinates (1-based) to the original video frame pixel coordinates. Essential for building interactive tools like remote desktops.

#### Utility Functions
- **`termvis.quick_play(source=0)`**
  A high-level function to quickly start a camera or video file preview with basic controls ('q' to quit).

### Key Capabilities

*   **Headless Remote Desktop**: See `examples/remote_desktop.py`. Mirror your physical display into SSH with mouse support.
*   **Lossless Recording**: The `.lzdx` format provides bit-perfect reconstruction of terminal pixels with massive space savings.
*   **Fourier Sharpening**: Boost high-frequency details on the fly to make text and edges pop in terminal resolutions.

### Examples 💡

Check the `examples/` directory for more:
- `basic_demo.py`: Simple rendering loop.
- `interactive_painter.py`: Draw on a high-res canvas using terminal mouse clicks.
- `remote_control_demo.py`: Python-side mouse event parsing and coordinate mapping.

### Technical Deep Dive
- **Rendering**: Uses 24-bit ANSI escape codes and the Half-Block (`▀`) character to effectively double the vertical resolution.
- **Compression**: XOR consecutive frames and Zlib the result. Backgrounds remain static, leading to huge compression ratios.
- **Engine**: Sampling and bit-mangling handled by a dedicated Rust core via PyO3.

### License
MIT
