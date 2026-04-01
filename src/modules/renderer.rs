use crossterm::terminal;
use opencv::{core, imgproc, prelude::*};
use std::io::{self, BufWriter, Write};

/// High-performance terminal renderer using Half-Block characters.
pub struct TerminalRenderer {
    handle: BufWriter<io::StdoutLock<'static>>,
}

impl TerminalRenderer {
    pub fn new() -> Self {
        let stdout = Box::leak(Box::new(io::stdout()));
        let handle = BufWriter::with_capacity(1024 * 1024, stdout.lock());
        Self { handle }
    }

    pub fn get_terminal_size(&self) -> (i32, i32) {
        let (term_w, term_h) = terminal::size().unwrap_or((80, 24));
        (term_w as i32, term_h as i32)
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
            imgproc::INTER_AREA, 
        )?;
        Ok(resized)
    }

    /// Renders the provided character map to the terminal using ANSI truecolor sequences.
    pub fn render_character_map(&mut self, character_map: &core::Mat) -> opencv::Result<()> {
        let mut rgb = core::Mat::default();
        imgproc::cvt_color_def(character_map, &mut rgb, imgproc::COLOR_BGR2RGB)?;

        let term_w = rgb.cols();
        let display_h = rgb.rows();

        write!(self.handle, "\x1B[H").unwrap();

        for y in (0..display_h).step_by(2) {
            for x in 0..term_w {
                let pixel1 = rgb.at_2d::<core::Vec3b>(y, x)?;
                let (r1, g1, b1) = (pixel1[0], pixel1[1], pixel1[2]);
                let (r2, g2, b2) = if y + 1 < display_h {
                    let pixel2 = rgb.at_2d::<core::Vec3b>(y + 1, x)?;
                    (pixel2[0], pixel2[1], pixel2[2])
                } else { (0, 0, 0) };

                write!(self.handle, "\x1B[38;2;{};{};{};48;2;{};{};{}m▀", r1, g1, b1, r2, g2, b2).unwrap();
            }
            write!(self.handle, "\x1B[K\r\n").unwrap();
        }

        write!(self.handle, "\x1B[0m").unwrap();
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
