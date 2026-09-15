use crate::frame::RawFrame;
use std::fmt;
use std::io::{self, Read};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct VideoCaptureInternalError{typ: i16, pub txt: String}
impl fmt::Display for VideoCaptureInternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.typ {
            1 => write!(f, "Invalid device identifier: {}", self.txt),
            2 => write!(f, "Can't probe video: {}", self.txt),
            _ => write!(f, "Video capture error: {}", self.txt)
        }
    }
}

#[derive(Debug)]
pub enum VideoCaptureError {
    VideoError(VideoCaptureInternalError),
    IOError(io::Error),
}

impl fmt::Display for VideoCaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VideoCaptureError::VideoError(e) => write!(f, "{}", e),
            VideoCaptureError::IOError(e) => write!(f, "{}", e),
        }
    }
}

impl From<VideoCaptureInternalError> for VideoCaptureError {
    fn from(e: VideoCaptureInternalError) -> Self {
        VideoCaptureError::VideoError(e)
    }
}

impl From<io::Error> for VideoCaptureError {
    fn from(e: io::Error) -> Self {
        VideoCaptureError::IOError(e)
    }
}

pub struct VideoCapture {
    process: VideoCaptureProcess,
    stdout: ChildStdout,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    row_stride: usize,
}

#[derive(Clone)]
pub struct VideoCaptureProcess {
    child: Arc<Mutex<Child>>,
}

impl VideoCaptureProcess {
    pub fn stop(&self) {
        let mut child = self.child.lock().unwrap_or_else(|err| err.into_inner());
        if let Ok(Some(_)) = child.try_wait() {
            return;
        }
        // Give GStreamer time to release a CSI camera before forcing termination.
        #[cfg(unix)]
        {
            let _ = nix::sys::signal::kill(nix::unistd::Pid::from_raw(child.id() as i32), nix::sys::signal::Signal::SIGINT);
            let deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < deadline {
                match child.try_wait() {
                    Ok(Some(_)) => return,
                    Ok(None) => thread::sleep(Duration::from_millis(20)),
                    Err(_) => break,
                }
            }
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}

impl Drop for VideoCaptureProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

impl VideoCapture {
    pub fn process(&self) -> VideoCaptureProcess {
        self.process.clone()
    }

    pub fn read_frame(&mut self) -> Result<Option<RawFrame>, VideoCaptureError> {
        let row_size = self.width as usize * 3;
        let mut frame = RawFrame {
            data: vec![0; self.row_stride * self.height as usize],
            width: self.width,
            height: self.height,
        };
        let mut read_bytes = 0;
        while read_bytes < frame.data.len() {
            match self.stdout.read(&mut frame.data[read_bytes..]) {
                Ok(0) => {
                    if read_bytes != 0 {
                        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Incomplete video frame").into());
                    }
                    return Ok(None);
                },
                Ok(count) => read_bytes += count,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(err.into()),
            }
        }
        // GStreamer aligns BGR rows to four bytes; RawFrame has no padding.
        if self.row_stride != row_size {
            for row in 1..self.height as usize {
                let start = row * self.row_stride;
                frame.data.copy_within(start..start + row_size, row * row_size);
            }
            frame.data.truncate(row_size * self.height as usize);
        }
        Ok(Some(frame))
    }
}

pub fn get_video_capture(video_src: &str, typ: String) -> Result<VideoCapture, VideoCaptureError> {
    let mut source = video_src.to_string();
    if typ != "rtsp" && !source.starts_with("/dev/video") {
        let device_id = source.parse::<u32>().map_err(|err| {
            VideoCaptureInternalError{typ: 1, txt: format!("{}. Err: {}", source, err)}
        })?;
        source = format!("/dev/video{}", device_id);
    }
    let gstreamer = source.contains(" ! ") && !source.starts_with("rtsp://") && !source.starts_with("rtsps://");
    let mut command;
    let (width, height, fps) = if gstreamer {
        let width = gst_value(&source, "width", "int").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        let height = gst_value(&source, "height", "int").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        let fps = gst_value(&source, "framerate", "fraction").map(|s| parse_frame_rate(&s)).unwrap_or(30.0);
        command = Command::new("gst-launch-1.0");
        command.arg("-q");
        let mut segments: Vec<&str> = source.split(" ! ").collect();
        if let Some(last) = segments.last() {
            let element = last.split_whitespace().next().unwrap_or("");
            if ["appsink", "fdsink", "fakesink", "filesink", "autovideosink"].contains(&element) {
                segments.pop();
            }
        }
        for (index, segment) in segments.iter().enumerate() {
            if index > 0 {
                command.arg("!");
            }
            if segment.contains("=(") || segment.trim().starts_with("video/") {
                command.arg(segment.split_whitespace().collect::<String>());
            } else {
                command.args(segment.split_whitespace());
            }
        }
        command.args(["!", "videoconvert", "!", "video/x-raw,format=BGR", "!", "fdsink", "fd=1"]);
        (width, height, fps)
    } else {
        let mut probe = Command::new("ffprobe");
        configure_input(&mut probe, &source);
        let output = probe.args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height,r_frame_rate", "-of", "json", &source]).output()?;
        if !output.status.success() {
            return Err(VideoCaptureInternalError{typ: 2, txt: String::from_utf8_lossy(&output.stderr).to_string()}.into());
        }
        let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|err| {
            VideoCaptureInternalError{typ: 2, txt: err.to_string()}
        })?;
        let stream = &metadata["streams"][0];
        let width = stream["width"].as_u64().and_then(|value| u32::try_from(value).ok()).unwrap_or(0);
        let height = stream["height"].as_u64().and_then(|value| u32::try_from(value).ok()).unwrap_or(0);
        let fps = parse_frame_rate(stream["r_frame_rate"].as_str().unwrap_or("30/1"));
        command = Command::new("ffmpeg");
        configure_input(&mut command, &source);
        command.args(["-nostdin", "-v", "error", "-i", &source, "-map", "0:v:0", "-an", "-sn", "-dn", "-fps_mode", "passthrough", "-f", "rawvideo", "-pix_fmt", "bgr24", "pipe:1"]);
        (width, height, fps)
    };
    let row_stride = if gstreamer { (width as usize * 3 + 3) & !3 } else { width as usize * 3 };
    if width == 0 || height == 0 || width > i32::MAX as u32 || height > i32::MAX as u32 || row_stride.checked_mul(height as usize).is_none() {
        return Err(VideoCaptureInternalError{typ: 2, txt: format!("Invalid video dimensions: {}x{}", width, height)}.into());
    }
    let mut child = command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn()?;
    let stdout = child.stdout.take().expect("Video capture stdout is not piped");
    Ok(VideoCapture {
        process: VideoCaptureProcess{child: Arc::new(Mutex::new(child))},
        stdout,
        width,
        height,
        fps,
        row_stride,
    })
}

fn configure_input(command: &mut Command, source: &str) {
    if source.starts_with("rtsp://") || source.starts_with("rtsps://") {
        command.args(["-rtsp_transport", "tcp"]);
    } else if source.starts_with("/dev/video") {
        command.args(["-f", "v4l2"]);
    }
}

fn gst_value(pipeline: &str, key: &str, typ: &str) -> Option<String> {
    let compact = pipeline.split_whitespace().collect::<String>();
    let pattern = format!("{}=({})", key, typ);
    let start = compact.rfind(&pattern)? + pattern.len();
    Some(compact[start..].chars().take_while(|c| c.is_ascii_digit() || *c == '/').collect())
}

fn parse_frame_rate(value: &str) -> f32 {
    let fps = match value.split_once('/') {
        Some((numerator, denominator)) => numerator.parse::<f32>().unwrap_or(0.0) / denominator.parse::<f32>().unwrap_or(0.0),
        None => value.parse::<f32>().unwrap_or(0.0),
    };
    if fps.is_finite() && fps > 0.0 { fps } else { 30.0 }
}
