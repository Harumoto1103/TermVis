use crossterm::terminal;
use opencv::{core, imgproc, prelude::*};
use std::io::{self, BufWriter, Write};

/// High-performance terminal renderer using Half-Block characters.
pub struct TerminalRenderer {
    handle: BufWriter<io::StdoutLock<'static>>,
    output_buf: Vec<u8>,
    last_cells: Vec<u64>,
    last_term_w: i32,
    last_term_h: i32,
    frame_count: u32,
}

impl TerminalRenderer {
    pub fn new() -> Self {
        let stdout = Box::leak(Box::new(io::stdout()));
        let handle = BufWriter::with_capacity(1024 * 1024, stdout.lock());
        Self {
            handle,
            output_buf: Vec::with_capacity(4 * 1024 * 1024),
            last_cells: Vec::new(),
            last_term_w: 0,
            last_term_h: 0,
            frame_count: 0,
        }
    }

    pub fn get_terminal_size(&mut self) -> (i32, i32) {
        self.frame_count = self.frame_count.wrapping_add(1);
        if self.last_term_w == 0 || self.frame_count % 30 == 0 {
            let (w, h) = terminal::size().unwrap_or((80, 24));
            self.last_term_w = w as i32;
            self.last_term_h = h as i32;
        }
        (self.last_term_w, self.last_term_h)
    }

    /// Prepares a character map by resizing the frame to fit the terminal dimensions.
    pub fn prepare_character_map(&self, frame: &core::Mat, term_w: i32, term_h: i32) -> opencv::Result<core::Mat> {
        let display_h = std::cmp::max(2, (term_h - 1) * 2);
        let mut resized = core::Mat::default();
        imgproc::resize(
            frame,
            &mut resized,
            core::Size::new(term_w, display_h),
            0.0,
            0.0,
            imgproc::INTER_LINEAR,
        )?;
        Ok(resized)
    }

    /// Renders the provided character map to the terminal using ANSI truecolor sequences.
    /// Input Mat is BGR (no conversion needed — B=[0], G=[1], R=[2]).
    pub fn render_character_map(&mut self, character_map: &core::Mat) -> opencv::Result<()> {
        let term_w = character_map.cols() as usize;
        let display_h = character_map.rows() as usize;
        let char_rows = display_h / 2;
        let total_cells = char_rows * term_w;

        // Rebuild last_cells if dimensions changed
        if self.last_cells.len() != total_cells {
            self.last_cells.clear();
            self.last_cells.resize(total_cells, u64::MAX);
        }

        // Get raw pointer and step for the character map (BGR, CV_8UC3)
        let data_ptr = character_map.data() as *const u8;
        let step = (character_map.cols() * 3) as usize; // contiguous Mat

        // Pass 1: compute new cells and count dirty
        let mut new_cells = vec![0u64; total_cells];
        let mut dirty_count = 0usize;

        for row in 0..char_rows {
            let y0 = row * 2;
            let y1 = y0 + 1;
            let row0_base = y0 * step;
            let row1_base = y1 * step;
            for col in 0..term_w {
                let off0 = row0_base + col * 3;
                let (b1, g1, r1) = unsafe {
                    (*data_ptr.add(off0), *data_ptr.add(off0 + 1), *data_ptr.add(off0 + 2))
                };
                let (b2, g2, r2) = if y1 < display_h {
                    let off1 = row1_base + col * 3;
                    unsafe {
                        (*data_ptr.add(off1), *data_ptr.add(off1 + 1), *data_ptr.add(off1 + 2))
                    }
                } else {
                    (0, 0, 0)
                };
                let cell = pack_cell(r1, g1, b1, r2, g2, b2);
                let idx = row * term_w + col;
                new_cells[idx] = cell;
                if cell != self.last_cells[idx] {
                    dirty_count += 1;
                }
            }
        }

        let buf = &mut self.output_buf;
        buf.clear();

        if dirty_count * 4 >= total_cells * 3 {
            // Full redraw: write all cells sequentially
            buf.extend_from_slice(b"\x1b[H");
            for row in 0..char_rows {
                for col in 0..term_w {
                    write_cell(buf, new_cells[row * term_w + col]);
                }
                buf.extend_from_slice(b"\x1b[K\r\n");
            }
            buf.extend_from_slice(b"\x1b[0m");
        } else {
            // Dirty-only: batch consecutive dirty cells per row with cursor positioning
            for row in 0..char_rows {
                let mut col = 0usize;
                while col < term_w {
                    let idx = row * term_w + col;
                    if new_cells[idx] != self.last_cells[idx] {
                        write_cursor_pos(buf, row + 1, col + 1);
                        while col < term_w {
                            let idx2 = row * term_w + col;
                            if new_cells[idx2] == self.last_cells[idx2] {
                                break;
                            }
                            write_cell(buf, new_cells[idx2]);
                            col += 1;
                        }
                    } else {
                        col += 1;
                    }
                }
            }
            buf.extend_from_slice(b"\x1b[0m");
        }

        // Swap in new_cells
        self.last_cells.copy_from_slice(&new_cells);

        self.handle.write_all(buf).unwrap();
        self.handle.flush().unwrap();
        Ok(())
    }

    /// Hides the cursor and switches to the alternate terminal buffer.
    pub fn hide_cursor(&self) {
        print!("\x1B[?25l\x1B[?1049h\x1B[H");
        io::stdout().flush().unwrap();
    }

    /// Switches back to the main terminal buffer and restores the cursor.
    pub fn show_cursor(&self) {
        print!("\x1B[?1049l\x1B[?25h");
        io::stdout().flush().unwrap();
    }
}

#[inline(always)]
fn pack_cell(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) -> u64 {
    (r1 as u64)
        | ((g1 as u64) << 8)
        | ((b1 as u64) << 16)
        | ((r2 as u64) << 24)
        | ((g2 as u64) << 32)
        | ((b2 as u64) << 40)
}

#[inline(always)]
fn write_cell(buf: &mut Vec<u8>, cell: u64) {
    let r1 = cell as u8;
    let g1 = (cell >> 8) as u8;
    let b1 = (cell >> 16) as u8;
    let r2 = (cell >> 24) as u8;
    let g2 = (cell >> 32) as u8;
    let b2 = (cell >> 40) as u8;
    buf.extend_from_slice(b"\x1b[38;2;");
    write_u8_dec(buf, r1);
    buf.push(b';');
    write_u8_dec(buf, g1);
    buf.push(b';');
    write_u8_dec(buf, b1);
    buf.extend_from_slice(b";48;2;");
    write_u8_dec(buf, r2);
    buf.push(b';');
    write_u8_dec(buf, g2);
    buf.push(b';');
    write_u8_dec(buf, b2);
    buf.push(b'm');
    buf.extend_from_slice(b"\xe2\x96\x80"); // ▀ in UTF-8
}

#[inline(always)]
fn write_cursor_pos(buf: &mut Vec<u8>, row: usize, col: usize) {
    buf.extend_from_slice(b"\x1b[");
    write_usize_dec(buf, row);
    buf.push(b';');
    write_usize_dec(buf, col);
    buf.push(b'H');
}

#[inline(always)]
fn write_u8_dec(buf: &mut Vec<u8>, v: u8) {
    if v >= 100 {
        buf.push(b'0' + v / 100);
        buf.push(b'0' + (v / 10) % 10);
        buf.push(b'0' + v % 10);
    } else if v >= 10 {
        buf.push(b'0' + v / 10);
        buf.push(b'0' + v % 10);
    } else {
        buf.push(b'0' + v);
    }
}

#[inline(always)]
fn write_usize_dec(buf: &mut Vec<u8>, v: usize) {
    if v >= 1000 {
        buf.push(b'0' + (v / 1000) as u8);
        buf.push(b'0' + ((v / 100) % 10) as u8);
        buf.push(b'0' + ((v / 10) % 10) as u8);
        buf.push(b'0' + (v % 10) as u8);
    } else if v >= 100 {
        buf.push(b'0' + (v / 100) as u8);
        buf.push(b'0' + ((v / 10) % 10) as u8);
        buf.push(b'0' + (v % 10) as u8);
    } else if v >= 10 {
        buf.push(b'0' + (v / 10) as u8);
        buf.push(b'0' + (v % 10) as u8);
    } else {
        buf.push(b'0' + v as u8);
    }
}
