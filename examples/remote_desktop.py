import cv2
import numpy as np
import time
import sys
import tty
import termios
import threading
import queue
import select
import os
from termvis import TermVis

# FIX: Set DISPLAY for Linux SSH sessions before importing GUI libraries
if sys.platform.startswith('linux') and 'DISPLAY' not in os.environ:
    print("No DISPLAY found. Defaulting to :0.0 for local X session...")
    os.environ['DISPLAY'] = ':0.0'

try:
    from mss import mss
    import pyautogui
except ImportError as e:
    print(f"Missing dependencies: {e}")
    print("Please run: pip install mss pyautogui")
    sys.exit(1)
except Exception as e:
    print(f"Error initializing GUI libraries: {e}")
    print("\nTroubleshooting for Linux/SSH:")
    print("1. Make sure you are logged into a graphical session on the remote machine.")
    print("2. Try running 'xhost +local:$(whoami)' on the physical machine's terminal.")
    sys.exit(1)

# Disable pyautogui safety delays for better responsiveness
pyautogui.PAUSE = 0
pyautogui.FAILSAFE = False

def run_remote_desktop():
    print("--- TermVis Remote Desktop (Beta) ---")
    print("Connecting to main screen...")
    
    # ANSI sequences for mouse/keyboard capture
    ENABLE_MOUSE = "\x1b[?1000h\x1b[?1006h"
    DISABLE_MOUSE = "\x1b[?1000l\x1b[?1006l"

    try:
        sct = mss()
        monitor = sct.monitors[1] # Primary monitor
    except Exception as e:
        print(f"Failed to capture screen: {e}")
        print("Note: On macOS, your terminal might need 'Screen Recording' permissions.")
        return
    
    event_queue = queue.Queue()
    stop_event = threading.Event()

    def input_worker():
        """Background thread to parse terminal stdin stream."""
        while not stop_event.is_set():
            try:
                r, _, _ = select.select([sys.stdin], [], [], 0.1)
                if not r: continue
                
                char = sys.stdin.read(1)
                if char.lower() == 'q':
                    event_queue.put(('command', 'quit'))
                elif char == '\x1b':
                    seq = sys.stdin.read(2)
                    if seq == '[<':
                        full_seq = ""
                        while True:
                            c = sys.stdin.read(1)
                            full_seq += c
                            if c in 'Mm': break
                        event_queue.put(('mouse', full_seq))
                else:
                    event_queue.put(('key', char))
            except: break

    with TermVis() as tv:
        # Prepare terminal environment
        sys.stdout.write(ENABLE_MOUSE)
        sys.stdout.flush()
        old_settings = termios.tcgetattr(sys.stdin)
        tty.setraw(sys.stdin)
        
        thread = threading.Thread(target=input_worker, daemon=True)
        thread.start()
        
        try:
            while True:
                # 2. Grab physical screen
                screenshot = sct.grab(monitor)
                frame = np.array(screenshot)
                frame = cv2.cvtColor(frame, cv2.COLOR_BGRA2BGR)
                
                # 3. Handle reverse-control events from terminal
                while not event_queue.empty():
                    ev_type, val = event_queue.get_nowait()
                    if ev_type == 'command' and val == 'quit':
                        raise KeyboardInterrupt
                    
                    elif ev_type == 'mouse':
                        parts = val[:-1].split(';')
                        if len(parts) == 3:
                            col, row = int(parts[1]), int(parts[2])
                            # Map terminal click to physical screen pixel coordinates
                            mx, my = tv.map_coords(col, row)
                            
                            # Apply monitor offsets
                            screen_x = monitor["left"] + mx
                            screen_y = monitor["top"] + my
                            
                            if val.endswith('M'): # Mouse Down
                                try:
                                    pyautogui.click(screen_x, screen_y)
                                    # Draw feedback on the frame
                                    cv2.circle(frame, (mx, my), 50, (0, 0, 255), 5)
                                except Exception as e:
                                    print(f"\rClick failed: {e}", end="")
                    
                    elif ev_type == 'key':
                        try:
                            pyautogui.write(val)
                        except: pass

                # 4. Render back to terminal
                cv2.putText(frame, f"REMOTE DESKTOP | {monitor['width']}x{monitor['height']}", 
                            (50, 100), cv2.FONT_HERSHEY_SIMPLEX, 2, (0, 255, 0), 5)
                
                tv.render(frame)
                time.sleep(0.01)
                
        except KeyboardInterrupt: pass
        finally:
            stop_event.set()
            termios.tcsetattr(sys.stdin, termios.TCSADRAIN, old_settings)
            sys.stdout.write(DISABLE_MOUSE)
            sys.stdout.flush()

if __name__ == "__main__":
    run_remote_desktop()
