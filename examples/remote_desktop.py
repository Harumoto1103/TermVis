import cv2
import numpy as np
import time
import sys
import tty
import termios
import threading
import queue
import select
from mss import mss
import pyautogui
from termvis import TermVis

# 关闭 pyautogui 的安全延迟以提高响应速度
pyautogui.PAUSE = 0
pyautogui.FAILSAFE = False

def run_remote_desktop():
    print("--- TermVis 终端远程桌面 (Beta) ---")
    print("正在连接主屏幕...")
    
    # 1. 鼠标/键盘 捕获 ANSI 指令
    ENABLE_MOUSE = "\x1b[?1000h\x1b[?1006h"
    DISABLE_MOUSE = "\x1b[?1000l\x1b[?1006l"

    sct = mss()
    monitor = sct.monitors[1] # 主显示器
    
    event_queue = queue.Queue()
    stop_event = threading.Event()

    def input_worker():
        """专门解析终端输入流的后台线程"""
        while not stop_event.is_set():
            try:
                # 检查是否有数据可读
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
                    # 模拟简单按键
                    event_queue.put(('key', char))
            except: break

    with TermVis() as tv:
        # 准备终端环境
        sys.stdout.write(ENABLE_MOUSE)
        sys.stdout.flush()
        old_settings = termios.tcgetattr(sys.stdin)
        tty.setraw(sys.stdin)
        
        thread = threading.Thread(target=input_worker, daemon=True)
        thread.start()
        
        try:
            while True:
                # 2. 截取物理屏幕
                screenshot = sct.grab(monitor)
                # 转换为 BGR (OpenCV 格式)
                frame = np.array(screenshot)
                frame = cv2.cvtColor(frame, cv2.COLOR_BGRA2BGR)
                
                # 3. 处理来自终端的反向控制事件
                while not event_queue.empty():
                    ev_type, val = event_queue.get_nowait()
                    if ev_type == 'command' and val == 'quit':
                        raise KeyboardInterrupt
                    
                    elif ev_type == 'mouse':
                        parts = val[:-1].split(';')
                        if len(parts) == 3:
                            col, row = int(parts[1]), int(parts[2])
                            # 将终端点击映射到真实的物理屏幕像素坐标
                            mx, my = tv.map_coords(col, row)
                            
                            # 映射到 mss 的绝对坐标 (考虑显示器偏移)
                            screen_x = monitor["left"] + mx
                            screen_y = monitor["top"] + my
                            
                            if val.endswith('M'): # 鼠标按下
                                pyautogui.click(screen_x, screen_y)
                                # 在反馈画面上画个圈，提示点击成功
                                cv2.circle(frame, (mx, my), 50, (0, 0, 255), 5)
                    
                    elif ev_type == 'key':
                        # 简单的字符输入模拟
                        pyautogui.write(val)

                # 4. 将屏幕画面渲染回终端
                # 在画面顶部加个提示
                cv2.putText(frame, f"REMOTE DESKTOP | {monitor['width']}x{monitor['height']}", 
                            (50, 100), cv2.FONT_HERSHEY_SIMPLEX, 2, (0, 255, 0), 5)
                
                tv.render(frame)
                
                # 控制帧率，避免过度消耗 CPU
                time.sleep(0.01)
                
        except KeyboardInterrupt: pass
        finally:
            stop_event.set()
            termios.tcsetattr(sys.stdin, termios.TCSADRAIN, old_settings)
            sys.stdout.write(DISABLE_MOUSE)
            sys.stdout.flush()

if __name__ == "__main__":
    run_remote_desktop()
