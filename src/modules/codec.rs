use opencv::{core, prelude::*};
use std::io::{Read, Write};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;

/// Handles lossless compression and frequency-domain image enhancement.
pub struct VideoCodec;

impl VideoCodec {
    /// Compresses a character map using Zlib DEFLATE algorithm.
    /// Uses raw pointer access for zero-copy extraction.
    pub fn compress_lossless(character_map: &core::Mat) -> opencv::Result<Vec<u8>> {
        let rows = character_map.rows() as usize;
        let cols = character_map.cols() as usize;
        let row_bytes = cols * 3;

        let data_ptr = unsafe { character_map.data() as *const u8 };
        // For contiguous Mats (output of resize/new_rows_cols_with_default), step == cols * elemSize
        let step = row_bytes; // contiguous assumption

        let total_size = rows * row_bytes;
        let mut raw_data = Vec::with_capacity(total_size);

        // Copy all rows; since Mat is contiguous we can do it in one shot
        let slice = unsafe { std::slice::from_raw_parts(data_ptr, rows * step) };
        raw_data.extend_from_slice(slice);

        Self::compress_raw(&raw_data)
            .map_err(|e| opencv::Error::new(opencv::core::StsError, e.to_string()))
    }

    /// Compresses a raw byte slice with zlib.
    pub fn compress_raw(data: &[u8]) -> std::io::Result<Vec<u8>> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        encoder.finish()
    }

    /// Decompress a Zlib-compressed byte stream back into a character map.
    /// Uses raw pointer access for zero-copy insertion.
    pub fn decompress_lossless(compressed: &[u8], rows: i32, cols: i32) -> opencv::Result<core::Mat> {
        let mut decoder = ZlibDecoder::new(compressed);
        let mut raw_data = Vec::new();
        decoder.read_to_end(&mut raw_data).unwrap();

        let mut mat = core::Mat::new_rows_cols_with_default(
            rows, cols, core::CV_8UC3, core::Scalar::all(0.0),
        )?;

        let row_bytes = cols as usize * 3;
        for y in 0..rows as usize {
            let src = &raw_data[y * row_bytes..(y + 1) * row_bytes];
            let dst_ptr = mat.ptr_mut(y as i32)? as *mut u8;
            unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), dst_ptr, row_bytes); }
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

            let rows = dft_mat.rows() as usize;
            let cols = dft_mat.cols() as usize;

            // step1(0) for CV_32FC2 gives step in elemSize1 (f32) units = cols * 2 for contiguous
            let dft_step = cols * 2; // CV_32FC2: 2 floats per pixel, contiguous

            let dft_data = unsafe {
                std::slice::from_raw_parts_mut(
                    dft_mat.ptr_mut(0)? as *mut f32,
                    rows * dft_step,
                )
            };

            for y in 0..rows {
                for x in 0..cols {
                    let idx = y * dft_step + x * 2;
                    let dx = if x < cols / 2 { x } else { cols - x } as f32;
                    let dy = if y < rows / 2 { y } else { rows - y } as f32;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let gain = 1.0 + amount * (dist / (rows as f32 / 2.0));
                    dft_data[idx] *= gain;
                    dft_data[idx + 1] *= gain;
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
