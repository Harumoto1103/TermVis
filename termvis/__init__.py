import term_video_py
import cv2

class TermVis:
    """
    TermVis provides high-performance terminal video rendering and recording.
    
    It uses Rust for core rendering and compression logic, supporting lossless
    incremental recording (LZDX) and real-time DFT sharpening.
    """
    
    def __init__(self):
        self._inner = term_video_py.TermVis()

    def __enter__(self):
        """Enables alternate buffer and hides cursor."""
        self._inner.hide_cursor()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Disables alternate buffer and restores cursor."""
        self._inner.show_cursor()
        self._inner.stop_recording()

    def render(self, frame_bgr):
        """
        Renders an OpenCV-style BGR frame to the terminal.
        Automatically converts to RGB and handles terminal resizing.
        """
        frame_rgb = cv2.cvtColor(frame_bgr, cv2.COLOR_BGR2RGB)
        h, w, _ = frame_rgb.shape
        self._inner.render(frame_rgb.tobytes(), w, h)

    def map_coords(self, terminal_col, terminal_row):
        """
        Maps 1-based terminal coordinates (column, row) to the original frame pixel coordinates (x, y).
        Useful for implementing interactive terminal-based UI or remote control.
        """
        info = self._inner.get_mapping_info()
        if info['term_w'] == 0 or info['display_h'] == 0:
            return 0, 0
        
        # Convert to 0-based indices
        c = max(0, terminal_col - 1)
        r = max(0, terminal_row - 1)
        
        # Calculate mapping ratios
        rx = info['frame_w'] / info['term_w']
        ry = info['frame_h'] / info['display_h']
        
        mx = int(c * rx)
        # Each terminal row represents two pixels in height (Half-block)
        my = int((r * 2 + 0.5) * ry)
        
        # Clip boundaries
        mx = min(mx, info['frame_w'] - 1)
        my = min(my, info['frame_h'] - 1)
        
        return mx, my

    def start_recording(self, path):
        """Starts recording the rendered output to an LZDX file."""
        self._inner.start_recording(path)

    def stop_recording(self):
        """Stops the current recording session."""
        self._inner.stop_recording()

    def play_recorded(self, path, sharpen=0.3):
        """
        Plays back a recorded LZDX file.
        :param path: Path to the .lzdx file.
        :param sharpen: Strength of the DFT sharpening filter (0.0 to 2.0).
        """
        self._inner.play(path, sharpen)
