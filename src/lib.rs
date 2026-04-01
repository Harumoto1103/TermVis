pub mod modules;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use modules::renderer::TerminalRenderer;
use modules::recorder::VideoRecorder;
use opencv::core;
use opencv::prelude::*;

/// Python wrapper for the TermVis library.
#[pyclass(unsendable)]
pub struct TermVis {
    renderer: TerminalRenderer,
    recorder: VideoRecorder,
    writer: Option<std::io::BufWriter<std::fs::File>>,
    last_frame_size: (i32, i32),
}

#[pymethods]
impl TermVis {
    #[new]
    fn new() -> Self {
        Self {
            renderer: TerminalRenderer::new(),
            recorder: VideoRecorder::new(),
            writer: None,
            last_frame_size: (0, 0),
        }
    }

    fn start_recording(&mut self, path: String) -> PyResult<()> {
        let file = std::fs::File::create(path).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        let mut writer = std::io::BufWriter::new(file);
        self.recorder.write_header(&mut writer).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        self.writer = Some(writer);
        Ok(())
    }

    fn stop_recording(&mut self) { self.writer = None; }

    fn render(&mut self, data: Vec<u8>, width: i32, height: i32) -> PyResult<()> {
        self.last_frame_size = (width, height);
        let mut frame = unsafe {
            core::Mat::new_rows_cols(height, width, core::CV_8UC3)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?
        };
        let mat_slice = unsafe { std::slice::from_raw_parts_mut(frame.data_mut() as *mut u8, data.len()) };
        mat_slice.copy_from_slice(&data);

        let (term_w, term_h) = self.renderer.get_terminal_size();
        let char_map = self.renderer.prepare_character_map(&frame, term_w, term_h)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        self.renderer.render_character_map(&char_map)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        if let Some(ref mut writer) = self.writer {
            self.recorder.record(&char_map, term_w, term_h, writer)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        }
        Ok(())
    }

    fn get_mapping_info(&self, py: Python<'_>) -> PyResult<PyObject> {
        let (term_w, term_h) = self.renderer.get_terminal_size();
        let dict = PyDict::new_bound(py);
        dict.set_item("term_w", term_w)?;
        dict.set_item("term_h", term_h)?;
        let display_h = std::cmp::max(2, (term_h - 1) * 2);
        dict.set_item("display_h", display_h)?;
        dict.set_item("frame_w", self.last_frame_size.0)?;
        dict.set_item("frame_h", self.last_frame_size.1)?;
        Ok(dict.to_object(py))
    }

    fn play(&self, path: String, sharpen: f32) -> PyResult<()> {
        let mut renderer = TerminalRenderer::new();
        self.recorder.play(&path, &mut renderer, sharpen)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(())
    }

    fn hide_cursor(&self) { self.renderer.hide_cursor(); }
    fn show_cursor(&self) { self.renderer.show_cursor(); }

    fn poll_key(&self) -> PyResult<Option<String>> {
        use crossterm::event::{self, Event, KeyCode};
        if event::poll(std::time::Duration::from_millis(0)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                match key.code {
                    KeyCode::Char(c) => return Ok(Some(c.to_string())),
                    KeyCode::Esc => return Ok(Some("esc".to_string())),
                    KeyCode::Enter => return Ok(Some("enter".to_string())),
                    _ => {}
                }
            }
        }
        Ok(None)
    }
}

/// This name MUST match the [lib] name in Cargo.toml
#[pymodule]
fn _termvis(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<TermVis>()?;
    Ok(())
}
