use std::fs::File;
use std::io::{Read, Write, BufReader, BufWriter};
use crate::modules::codec::VideoCodec;
use crate::modules::renderer::TerminalRenderer;
use opencv::{core, imgproc, prelude::*};

const KEYFRAME_INTERVAL: u32 = 120; // keyframe every ~4s at 30fps

/// Manages recording and playback of terminal video using delta-xor encoding.
pub struct VideoRecorder {
    last_frame_data: Vec<u8>,
    last_rows: i32,
    last_cols: i32,
    frame_count: u32,
}

impl VideoRecorder {
    pub fn new() -> Self {
        Self {
            last_frame_data: Vec::new(),
            last_rows: 0,
            last_cols: 0,
            frame_count: 0,
        }
    }

    /// Records a frame using lossless Zlib compression and Delta-XOR for temporal redundancy.
    pub fn record(
        &mut self,
        character_map: &core::Mat,
        w: i32,
        h: i32,
        writer: &mut BufWriter<File>,
    ) -> opencv::Result<()> {
        let rows = character_map.rows();
        let cols = character_map.cols();
        let row_bytes = cols as usize * 3;
        let total_bytes = rows as usize * row_bytes;

        // Extract raw bytes from the Mat (contiguous)
        let data_ptr = unsafe { character_map.data() as *const u8 };
        let current_data: Vec<u8> =
            unsafe { std::slice::from_raw_parts(data_ptr, total_bytes) }.to_vec();

        let size_changed = rows != self.last_rows || cols != self.last_cols;
        let is_keyframe = self.last_frame_data.is_empty()
            || size_changed
            || self.frame_count % KEYFRAME_INTERVAL == 0;

        self.frame_count = self.frame_count.wrapping_add(1);

        let (frame_type, compressed_data) = if is_keyframe {
            let compressed = VideoCodec::compress_raw(&current_data)
                .map_err(|e| opencv::Error::new(opencv::core::StsError, e.to_string()))?;
            (0u8, compressed)
        } else {
            // XOR delta
            let mut delta = vec![0u8; total_bytes];
            for i in 0..total_bytes {
                delta[i] = current_data[i] ^ self.last_frame_data[i];
            }
            let compressed = VideoCodec::compress_raw(&delta)
                .map_err(|e| opencv::Error::new(opencv::core::StsError, e.to_string()))?;
            (1u8, compressed)
        };

        writer.write_all(&[frame_type]).unwrap();
        writer.write_all(&w.to_le_bytes()).unwrap();
        writer.write_all(&h.to_le_bytes()).unwrap();
        writer.write_all(&rows.to_le_bytes()).unwrap();
        writer.write_all(&cols.to_le_bytes()).unwrap();

        let data_len = compressed_data.len() as u32;
        writer.write_all(&data_len.to_le_bytes()).unwrap();
        writer.write_all(&compressed_data).unwrap();

        // Store current frame bytes, reuse allocation if same size
        if self.last_frame_data.len() == current_data.len() {
            self.last_frame_data.copy_from_slice(&current_data);
        } else {
            self.last_frame_data = current_data;
        }
        self.last_rows = rows;
        self.last_cols = cols;

        Ok(())
    }

    pub fn write_header(&self, writer: &mut BufWriter<File>) -> std::io::Result<()> {
        writer.write_all(b"LZDX")?;
        Ok(())
    }

    /// Plays back a recorded LZDX file.
    pub fn play(
        &self,
        path: &str,
        renderer: &mut TerminalRenderer,
        sharpen_amount: f32,
    ) -> opencv::Result<()> {
        let file = File::open(path)
            .map_err(|e| opencv::Error::new(opencv::core::StsError, e.to_string()))?;
        let mut reader = BufReader::new(file);

        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic).unwrap();
        if &magic != b"LZDX" {
            return Err(opencv::Error::new(opencv::core::StsError, "Invalid Format"));
        }

        let mut current_frame_data: Vec<u8> = Vec::new();
        let mut last_curr_w = 0i32;
        let mut last_curr_h = 0i32;

        renderer.hide_cursor();
        loop {
            let frame_start = std::time::Instant::now();

            let mut frame_type = [0u8; 1];
            if reader.read_exact(&mut frame_type).is_err() { break; }

            let mut meta = [0u8; 20];
            reader.read_exact(&mut meta).unwrap();
            let _rec_w = i32::from_le_bytes(meta[0..4].try_into().unwrap());
            let _rec_h = i32::from_le_bytes(meta[4..8].try_into().unwrap());
            let rows = i32::from_le_bytes(meta[8..12].try_into().unwrap());
            let cols = i32::from_le_bytes(meta[12..16].try_into().unwrap());
            let data_len = u32::from_le_bytes(meta[16..20].try_into().unwrap()) as usize;

            let mut data = vec![0u8; data_len];
            reader.read_exact(&mut data).unwrap();

            let decoded_map = VideoCodec::decompress_lossless(&data, rows, cols)?;

            let character_map = if frame_type[0] == 0 {
                // Keyframe: capture raw bytes for future delta
                let row_bytes = cols as usize * 3;
                let total_bytes = rows as usize * row_bytes;
                let data_ptr = unsafe { decoded_map.data() as *const u8 };
                let new_data = unsafe { std::slice::from_raw_parts(data_ptr, total_bytes) };
                if current_frame_data.len() == total_bytes {
                    current_frame_data.copy_from_slice(new_data);
                } else {
                    current_frame_data = new_data.to_vec();
                }
                decoded_map
            } else {
                // Delta frame: XOR with current_frame_data
                let row_bytes = cols as usize * 3;
                let total_bytes = rows as usize * row_bytes;

                // Get decoded delta bytes
                let delta_ptr = unsafe { decoded_map.data() as *const u8 };
                let delta_slice = unsafe { std::slice::from_raw_parts(delta_ptr, total_bytes) };

                // XOR into restored bytes
                let mut restored_data = vec![0u8; total_bytes];
                for i in 0..total_bytes {
                    restored_data[i] = delta_slice[i] ^ current_frame_data[i];
                }

                // Update current_frame_data
                if current_frame_data.len() == total_bytes {
                    current_frame_data.copy_from_slice(&restored_data);
                } else {
                    current_frame_data = restored_data.clone();
                }

                // Build Mat from restored bytes
                let mut restored_mat = core::Mat::new_rows_cols_with_default(
                    rows, cols, core::CV_8UC3, core::Scalar::all(0.0),
                )?;
                for y in 0..rows as usize {
                    let src = &current_frame_data[y * row_bytes..(y + 1) * row_bytes];
                    let dst_ptr = restored_mat.ptr_mut(y as i32)? as *mut u8;
                    unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), dst_ptr, row_bytes); }
                }
                restored_mat
            };

            let mut render_map = if sharpen_amount > 0.0 {
                VideoCodec::sharpen_with_dft(&character_map, sharpen_amount)?
            } else {
                character_map
            };

            // Cache terminal size — only call get_terminal_size (it throttles internally)
            let (curr_w, curr_h) = renderer.get_terminal_size();
            let curr_display_h = std::cmp::max(2, (curr_h - 1) * 2);

            // Only resize if terminal dimensions changed
            if curr_w != last_curr_w || curr_h != last_curr_h {
                last_curr_w = curr_w;
                last_curr_h = curr_h;
            }

            let render_rows = render_map.rows();
            let render_cols = render_map.cols();
            if curr_w != render_cols || curr_display_h != render_rows {
                let mut resized = core::Mat::default();
                imgproc::resize(
                    &render_map,
                    &mut resized,
                    core::Size::new(curr_w, curr_display_h),
                    0.0,
                    0.0,
                    imgproc::INTER_LINEAR,
                )?;
                render_map = resized;
            }

            renderer.render_character_map(&render_map)?;

            // Frame-rate throttling based on actual elapsed time
            let elapsed = frame_start.elapsed();
            let target = std::time::Duration::from_millis(33);
            if elapsed < target {
                std::thread::sleep(target - elapsed);
            }
        }
        renderer.show_cursor();
        Ok(())
    }
}
