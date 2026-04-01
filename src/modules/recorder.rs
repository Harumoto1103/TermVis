use std::fs::File;
use std::io::{Read, Write, BufReader, BufWriter};
use crate::modules::codec::VideoCodec;
use crate::modules::renderer::TerminalRenderer;
use opencv::{core, imgproc, prelude::*};

/// Manages recording and playback of terminal video using delta-xor encoding.
pub struct VideoRecorder {
    last_frame: Option<core::Mat>,
}

impl VideoRecorder {
    pub fn new() -> Self {
        Self { last_frame: None }
    }

    /// Records a frame using lossless Zlib compression and Delta-XOR for temporal redundancy.
    pub fn record(&mut self, character_map: &core::Mat, w: i32, h: i32, writer: &mut BufWriter<File>) -> opencv::Result<()> {
        let rows = character_map.rows();
        let cols = character_map.cols();
        
        let is_keyframe = self.last_frame.as_ref()
            .map_or(true, |last| last.rows() != rows || last.cols() != cols);

        let (frame_type, data_to_compress) = if is_keyframe {
            (0u8, character_map.clone())
        } else {
            let mut delta = core::Mat::default();
            core::bitwise_xor(character_map, self.last_frame.as_ref().unwrap(), &mut delta, &core::no_array())?;
            (1u8, delta)
        };

        let compressed_data = VideoCodec::compress_lossless(&data_to_compress)?;
        
        writer.write_all(&[frame_type]).unwrap();
        writer.write_all(&w.to_le_bytes()).unwrap();
        writer.write_all(&h.to_le_bytes()).unwrap();
        writer.write_all(&rows.to_le_bytes()).unwrap();
        writer.write_all(&cols.to_le_bytes()).unwrap();
        
        let data_len = compressed_data.len() as u32;
        writer.write_all(&data_len.to_le_bytes()).unwrap();
        writer.write_all(&compressed_data).unwrap();

        self.last_frame = Some(character_map.clone());
        Ok(())
    }

    pub fn write_header(&self, writer: &mut BufWriter<File>) -> std::io::Result<()> {
        writer.write_all(b"LZDX")?;
        Ok(())
    }

    /// Plays back a recorded LZDX file.
    pub fn play(&self, path: &str, renderer: &mut TerminalRenderer, sharpen_amount: f32) -> opencv::Result<()> {
        let file = File::open(path).map_err(|e| opencv::Error::new(opencv::core::StsError, e.to_string()))?;
        let mut reader = BufReader::new(file);
        
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic).unwrap();
        if &magic != b"LZDX" { return Err(opencv::Error::new(opencv::core::StsError, "Invalid Format")); }

        let mut current_frame: Option<core::Mat> = None;

        renderer.hide_cursor();
        loop {
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

            let mut character_map = if frame_type[0] == 0 {
                decoded_map
            } else {
                let mut restored = core::Mat::default();
                core::bitwise_xor(&decoded_map, current_frame.as_ref().unwrap(), &mut restored, &core::no_array())?;
                restored
            };

            current_frame = Some(character_map.clone());

            if sharpen_amount > 0.0 {
                character_map = VideoCodec::sharpen_with_dft(&character_map, sharpen_amount)?;
            }

            let (curr_w, curr_h) = renderer.get_terminal_size();
            let curr_display_h = std::cmp::max(2, (curr_h - 1) * 2);
            
            let mut render_map = character_map.clone();
            if curr_w != cols || curr_display_h != rows {
                let mut resized = core::Mat::default();
                imgproc::resize(&character_map, &mut resized, core::Size::new(curr_w, curr_display_h), 0.0, 0.0, imgproc::INTER_LINEAR)?;
                render_map = resized;
            }

            renderer.render_character_map(&render_map)?;
            std::thread::sleep(std::time::Duration::from_millis(33));
        }
        renderer.show_cursor();
        Ok(())
    }
}
