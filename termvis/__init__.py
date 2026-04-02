from ._termvis import TermVis as _TermVis
import cv2

class TermVis:
    """
    TermVis provides high-performance terminal video rendering and recording.
    """

    def __init__(self):
        self._inner = _TermVis()

    def __enter__(self):
        self._inner.hide_cursor()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self._inner.show_cursor()
        self._inner.stop_recording()

    def render(self, frame_bgr):
        """Renders an OpenCV-style BGR frame to the terminal."""
        h, w, _ = frame_bgr.shape
        self._inner.render(frame_bgr.tobytes(), w, h)

    def get_mapping_info(self):
        """Returns metadata for coordinate mapping."""
        return self._inner.get_mapping_info()

    def map_coords(self, terminal_col, terminal_row):
        """Maps 1-based terminal coordinates to original frame pixel coordinates."""
        info = self._inner.get_mapping_info()
        if info['term_w'] == 0 or info['display_h'] == 0:
            return 0, 0
        
        c = max(0, terminal_col - 1)
        r = max(0, terminal_row - 1)
        
        rx = info['frame_w'] / info['term_w']
        ry = info['frame_h'] / info['display_h']
        
        mx = int(c * rx)
        my = int((r * 2 + 0.5) * ry)
        
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
        """Plays back a recorded LZDX file."""
        self._inner.play(path, sharpen)

    def poll_key(self):
        """Polls for keyboard events (non-blocking)."""
        return self._inner.poll_key()

def quick_play(source=0):
    """Utility to quickly preview a video source in the terminal."""
    cap = cv2.VideoCapture(source)
    with TermVis() as tv:
        while True:
            ret, frame = cap.read()
            if not ret: break
            tv.render(frame)
            if tv.poll_key() == 'q': break
    cap.release()
