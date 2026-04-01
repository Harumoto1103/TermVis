import cv2
import numpy as np
import time
import sys
import tty
import termios
import threading
import queue
from termvis import TermVis

def run_painter_demo():
    # 1. 启动 TermVis 实例以获取比例
    tv_raw = TermVis()
    
    # 开启鼠标 ANSI 指令
    ENABLE_MOUSE = "\x1b[?1000h\x1b[?1006h"
    DISABLE_MOUSE = "\x1b[?1000l\x1b[?1006l"

    print("--- TermVis 比例校准涂鸦板 ---")
    print("正在根据您的终端窗口调整比例...")
    time.sleep(0.5)

    with tv_raw as tv:
        # 获取当前映射信息，用于创建“比例一致”的画布
        info = tv.get_mapping_info()
        # 终端的一个字符单元大约是正方形的 (Half-block 逻辑)
        # 所以我们将画布的分辨率设置为终端行列数的 10 倍，以保持极高精度
        canvas_w = info['term_w'] * 10
        canvas_h = info['display_h'] * 10
        
        # 确保画布不是 0
        canvas_w = max(canvas_w, 800)
        canvas_h = max(canvas_h, 400)
        
        canvas = np.zeros((canvas_h, canvas_w, 3), dtype=np.uint8)
        # 画背景网格
        for i in range(0, canvas_w, canvas_w // 10): cv2.line(canvas, (i, 0), (i, canvas_h), (30, 30, 30), 1)
        for i in range(0, canvas_h, canvas_h // 10): cv2.line(canvas, (0, i), (canvas_w, i), (30, 30, 30), 1)

        event_queue = queue.Queue()
        stop_event = threading.Event()

        def input_worker():
            while not stop_event.is_set():
                try:
                    char = sys.stdin.read(1)
                    if char == '\x1b':
                        seq = sys.stdin.read(2)
                        if seq == '[<':
                            full_seq = ""
                            while True:
                                c = sys.stdin.read(1)
                                full_seq += c
                                if c in 'Mm': break
                            event_queue.put(('mouse', full_seq))
                    elif char.lower() == 'q': event_queue.put(('key', 'q'))
                    elif char.lower() == 'c': event_queue.put(('key', 'c'))
                except: break

        sys.stdout.write(ENABLE_MOUSE)
        sys.stdout.flush()
        old_settings = termios.tcgetattr(sys.stdin)
        tty.setraw(sys.stdin)
        
        thread = threading.Thread(target=input_worker, daemon=True)
        thread.start()
        
        try:
            while True:
                # 处理输入
                while not event_queue.empty():
                    ev_type, val = event_queue.get_nowait()
                    if ev_type == 'key':
                        if val == 'q': raise KeyboardInterrupt
                        if val == 'c': canvas.fill(0)
                    elif ev_type == 'mouse':
                        parts = val[:-1].split(';')
                        if len(parts) == 3:
                            col, row = int(parts[1]), int(parts[2])
                            # 使用库提供的映射方法
                            mx, my = tv.map_coords(col, row)
                            
                            if val.endswith('M'): # 点击
                                color = (int(np.random.randint(100, 255)), 
                                         int(np.random.randint(100, 255)), 
                                         int(np.random.randint(100, 255)))
                                # 画圆
                                cv2.circle(canvas, (mx, my), canvas_w // 40, color, -1)
                                cv2.putText(canvas, f"Pos: {mx},{my}", (mx + 20, my), 
                                            cv2.FONT_HERSHEY_SIMPLEX, 0.6, color, 2)

                # 渲染
                # 即使画布分辨率很高，Rust 核心现在也会使用 INTER_AREA 平滑地压缩它
                tv.render(canvas)
                time.sleep(0.02)
                
        except KeyboardInterrupt: pass
        finally:
            stop_event.set()
            termios.tcsetattr(sys.stdin, termios.TCSADRAIN, old_settings)
            sys.stdout.write(DISABLE_MOUSE)
            sys.stdout.flush()

if __name__ == "__main__":
    run_painter_demo()
