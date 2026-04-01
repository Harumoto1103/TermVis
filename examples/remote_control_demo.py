import cv2
import numpy as np
import time
import sys
import tty
import termios
from termvis import TermVis

def run_remote_control_demo():
    print("--- TermVis Python层鼠标控制测试 ---")
    print("本测试演示如何在 Python 脚本中捕获鼠标并映射到原始帧像素。")
    print("1. 开启终端鼠标捕获模式...")
    
    # 模拟高分画布
    canvas_w, canvas_h = 1920, 1080
    canvas = np.zeros((canvas_h, canvas_w, 3), dtype=np.uint8)
    
    # Python 开启/关闭鼠标捕获的 ANSI 序列
    ENABLE_MOUSE = "\x1b[?1000h\x1b[?1003h\x1b[?1015h\x1b[?1006h"
    DISABLE_MOUSE = "\x1b[?1000l\x1b[?1003l\x1b[?1015l\x1b[?1006l"

    with TermVis() as tv:
        sys.stdout.write(ENABLE_MOUSE)
        sys.stdout.flush()
        
        # 将终端设为原始模式以读取鼠标字节流
        old_settings = termios.tcgetattr(sys.stdin)
        tty.setraw(sys.stdin)
        
        try:
            while True:
                # 绘制移动的背景增加动感
                t = time.time()
                frame = canvas.copy()
                cv2.putText(frame, f"Remote System Time: {t:.2f}", (50, 100), 
                            cv2.FONT_HERSHEY_SIMPLEX, 2.0, (0, 255, 0), 3)
                
                # 1. 尝试读取键盘/鼠标 (Python 解析层)
                # 使用 select 处理非阻塞读取
                import select
                if select.select([sys.stdin], [], [], 0)[0]:
                    data = sys.stdin.read(1)
                    if data == 'q': break
                    if data == '\x1b': # 可能是转义序列 (鼠标)
                        seq = sys.stdin.read(2)
                        if seq == '[<': # SGR 鼠标编码
                            full_seq = ""
                            while True:
                                char = sys.stdin.read(1)
                                full_seq += char
                                if char in 'Mm': break
                            
                            # 解析 SGR 格式: <button;col;row[Mm]
                            parts = full_seq[:-1].split(';')
                            if len(parts) == 3:
                                col, row = int(parts[1]), int(parts[2])
                                # 使用库提供的映射方法
                                mx, my = tv.map_coords(col - 1, row - 1)
                                
                                # 在画布上记录点击
                                cv2.circle(canvas, (mx, my), 30, (0, 0, 255), -1)
                                cv2.putText(canvas, f"HIT! {mx},{my}", (mx, my-40), 
                                            cv2.FONT_HERSHEY_SIMPLEX, 1.5, (255, 255, 255), 4)

                # 2. 渲染
                tv.render(frame)
                time.sleep(0.02)
                
        finally:
            # 恢复终端状态
            termios.tcsetattr(sys.stdin, termios.TCSADRAIN, old_settings)
            sys.stdout.write(DISABLE_MOUSE)
            sys.stdout.flush()

if __name__ == "__main__":
    run_remote_control_demo()
