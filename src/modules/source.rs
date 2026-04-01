use opencv::{core, videoio, prelude::*};

/// Trait defining a generic video source.
pub trait VideoSource {
    fn get_frame(&mut self) -> opencv::Result<Option<core::Mat>>;
    fn dimensions(&self) -> (i32, i32);
}

/// Video source using a physical camera.
pub struct CameraSource {
    cam: videoio::VideoCapture,
}

impl CameraSource {
    pub fn new(index: i32) -> opencv::Result<Self> {
        let cam = videoio::VideoCapture::new(index, videoio::CAP_ANY)?;
        Ok(Self { cam })
    }
}

impl VideoSource for CameraSource {
    fn get_frame(&mut self) -> opencv::Result<Option<core::Mat>> {
        let mut frame = core::Mat::default();
        if self.cam.read(&mut frame)? && !frame.empty() {
            let mut flipped = core::Mat::default();
            core::flip(&frame, &mut flipped, 1)?;
            Ok(Some(flipped))
        } else {
            Ok(None)
        }
    }

    fn dimensions(&self) -> (i32, i32) {
        let w = self.cam.get(videoio::CAP_PROP_FRAME_WIDTH).unwrap_or(0.0) as i32;
        let h = self.cam.get(videoio::CAP_PROP_FRAME_HEIGHT).unwrap_or(0.0) as i32;
        (w, h)
    }
}

/// Video source using a file path.
pub struct FileSource {
    cap: videoio::VideoCapture,
}

impl FileSource {
    pub fn new(path: &str) -> opencv::Result<Self> {
        let cap = videoio::VideoCapture::from_file(path, videoio::CAP_ANY)?;
        Ok(Self { cap })
    }
}

impl VideoSource for FileSource {
    fn get_frame(&mut self) -> opencv::Result<Option<core::Mat>> {
        let mut frame = core::Mat::default();
        if self.cap.read(&mut frame)? && !frame.empty() {
            Ok(Some(frame))
        } else {
            Ok(None)
        }
    }

    fn dimensions(&self) -> (i32, i32) {
        let w = self.cap.get(videoio::CAP_PROP_FRAME_WIDTH).unwrap_or(0.0) as i32;
        let h = self.cap.get(videoio::CAP_PROP_FRAME_HEIGHT).unwrap_or(0.0) as i32;
        (w, h)
    }
}
