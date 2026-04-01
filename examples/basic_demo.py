import cv2
import numpy as np
import time
from termvis import TermVis

def create_animation_frame(t):
    """生成一个简单的动态色块动画"""
    frame = np.zeros((120, 160, 3), dtype=np.uint8)
    # 动态颜色
    color = (int(127 + 127 * np.sin(t)), int(127 + 127 * np.cos(t)), 255)
    cv2.rectangle(frame, (20, 20), (140, 100), color, -1)
    cv2.putText(frame, f"TIME: {t:.1f}s", (30, 60), 
                cv2.FONT_HERSHEY_SIMPLEX, 0.5, (0, 0, 0), 2)
    return frame

def run_test():
    print("--- TermVis 极简模式测试 ---")
    print("1. 预览并录制 5 秒...")
    
    with TermVis() as tv:
        tv.start_recording("minimal_test.lzdx")
        start_time = time.time()
        
        while (time.time() - start_time) < 5:
            t = time.time() - start_time
            frame = create_animation_frame(t)
            
            # 纯净渲染 + 自动录制（因为调用了 start_recording）
            tv.render(frame)
            
            if tv.poll_key() == 'q':
                break
            time.sleep(0.03)
            
        tv.stop_recording()
        print("\n2. 录制完成。现在开始回放...")
        time.sleep(1)
        
        # 播放刚才录制的内容
        tv.play_recorded("minimal_test.lzdx", sharpen=0.5)

    print("\n测试结束！")

if __name__ == "__main__":
    run_test()
