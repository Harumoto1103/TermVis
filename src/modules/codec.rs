use opencv::{core, prelude::*};
use std::io::{Read, Write};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;

/// Handles lossless compression and frequency-domain image enhancement.
pub struct VideoCodec;

impl VideoCodec {
    /// Compresses a character map using Zlib DEFLATE algorithm.
    pub fn compress_lossless(character_map: &core::Mat) -> opencv::Result<Vec<u8>> {
        let rows = character_map.rows();
        let cols = character_map.cols();
        let total_size = (rows * cols * 3) as usize;
        
        let mut raw_data = vec![0u8; total_size];
        let mut idx = 0;
        for y in 0..rows {
            for x in 0..cols {
                let p = character_map.at_2d::<core::Vec3b>(y, x)?;
                raw_data[idx] = p[0];
                raw_data[idx+1] = p[1];
                raw_data[idx+2] = p[2];
                idx += 3;
            }
        }

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&raw_data).unwrap();
        Ok(encoder.finish().unwrap())
    }

    /// Decompress a Zlib-compressed byte stream back into a character map.
    pub fn decompress_lossless(compressed: &[u8], rows: i32, cols: i32) -> opencv::Result<core::Mat> {
        let mut decoder = ZlibDecoder::new(compressed);
        let mut raw_data = Vec::new();
        decoder.read_to_end(&mut raw_data).unwrap();

        let mut mat = core::Mat::new_rows_cols_with_default(rows, cols, core::CV_8UC3, core::Scalar::all(0.0))?;
        let mut idx = 0;
        for y in 0..rows {
            for x in 0..cols {
                let p = mat.at_2d_mut::<core::Vec3b>(y, x)?;
                p[0] = raw_data[idx];
                p[1] = raw_data[idx+1];
                p[2] = raw_data[idx+2];
                idx += 3;
            }
        }
        Ok(mat)
    }

    /// Enhances image details using Discrete Fourier Transform (DFT) sharpening.
    pub fn sharpen_with_dft(character_map: &core::Mat, amount: f32) -> opencv::Result<core::Mat> {
        if amount <= 0.0 { return Ok(character_map.clone()); }

        let mut channels = core::Vector::<core::Mat>::new();
        core::split(character_map, &mut channels)?;
        let mut sharpened_channels = core::Vector::<core::Mat>::new();

        for i in 0..3 {
            let channel = channels.get(i)?;
            let mut float_chan = core::Mat::default();
            channel.convert_to(&mut float_chan, core::CV_32F, 1.0, 0.0)?;

            let mut dft_mat = core::Mat::default();
            core::dft(&float_chan, &mut dft_mat, core::DFT_COMPLEX_OUTPUT, 0)?;

            let rows = dft_mat.rows();
            let cols = dft_mat.cols();
            for y in 0..rows {
                for x in 0..cols {
                    let val: &mut core::Vec2f = dft_mat.at_2d_mut::<core::Vec2f>(y, x)?;
                    let dx = if x < cols/2 { x } else { cols - x } as f32;
                    let dy = if y < rows/2 { y } else { rows - y } as f32;
                    let dist = (dx*dx + dy*dy).sqrt();
                    let gain = 1.0 + amount * (dist / (rows as f32 / 2.0));
                    val[0] *= gain;
                    val[1] *= gain;
                }
            }

            let mut idft_mat = core::Mat::default();
            core::dft(&dft_mat, &mut idft_mat, core::DFT_INVERSE | core::DFT_REAL_OUTPUT | core::DFT_SCALE, 0)?;

            let mut final_chan = core::Mat::default();
            idft_mat.convert_to(&mut final_chan, core::CV_8U, 1.0, 0.0)?;
            sharpened_channels.push(final_chan);
        }

        let mut merged = core::Mat::default();
        core::merge(&sharpened_channels, &mut merged)?;
        Ok(merged)
    }
}
