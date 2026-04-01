# TermVis

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
pip install maturin
maturin develop --release

# Run a quick camera test
python -c "import termvis; termvis.quick_play(0)"
```

### Key Capabilities

*   **Headless Remote Desktop**: Run the `remote_desktop.py` example to mirror your physical display into your SSH session. It supports mouse mapping, so clicking in your terminal actually clicks on the remote machine.
*   **Lossless Recording**: The `.lzdx` format isn't just a video; it's a bit-perfect reconstruction of your terminal pixels. Great for logging training sessions.
*   **Fourier Sharpening**: Use the `sharpen` parameter during playback to boost high-frequency details. It makes text and edges pop in terminal resolutions.
*   **Mouse-to-Pixel Mapping**: Precise coordinate transformation that lets you build interactive terminal UIs.

### Examples

Check the `examples/` directory:
- `basic_demo.py`: The "Hello World" of terminal rendering.
- `interactive_painter.py`: Test your mouse mapping by drawing on a 1080p canvas using terminal clicks.
- `remote_desktop.py`: Control your desktop from a terminal.

### Technical Deep Dive
- **Rendering**: Uses 24-bit ANSI escape codes and the Half-Block (`▀`) character to effectively double the vertical resolution.
- **Compression**: Instead of standard video codecs, we XOR consecutive frames and Zlib the result. Since terminal backgrounds are often static, this hits massive compression ratios while remaining CPU efficient.
- **Engine**: The sampling and bit-mangling are handled by a dedicated Rust crate linked via PyO3.

### License
MIT
