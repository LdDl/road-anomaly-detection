use super::*;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

struct Marker(PathBuf);

impl Marker {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("road-capture-{}", Uuid::new_v4())))
    }
}

impl Drop for Marker {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn capture(script: &str, marker: &Marker, reconnect: bool) -> VideoCapture {
    let mut command = Command::new("sh");
    command.args(["-c", script, "capture-test"]).arg(&marker.0);
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let (child, stdout) = spawn_capture(&mut command).unwrap();
    VideoCapture {
        process: VideoCaptureProcess{child: Arc::new(Mutex::new(child)), stopped: Arc::new(AtomicBool::new(false))},
        command,
        reconnect,
        stdout,
        stream_generation: 0,
        width: 1,
        height: 1,
        fps: 25.0,
        row_stride: 3,
    }
}

#[test]
fn file_eof_does_not_restart_capture() {
    let marker = Marker::new();
    let mut input = capture("printf abc", &marker, false);
    assert_eq!(input.read_frame().unwrap().unwrap().data, b"abc");
    assert!(input.read_frame().unwrap().is_none());
    assert_eq!(input.stream_generation, 0);
}

#[test]
fn incomplete_file_frame_remains_an_error() {
    let marker = Marker::new();
    let mut input = capture("printf ab", &marker, false);
    assert!(matches!(input.read_frame(), Err(VideoCaptureError::IOError(err)) if err.kind() == io::ErrorKind::UnexpectedEof));
}

#[test]
fn failed_capture_exit_is_reported() {
    let marker = Marker::new();
    let mut input = capture("exit 7", &marker, false);
    input.process.child.lock().unwrap().wait().unwrap();
    let error = input.read_frame().err().unwrap().to_string();
    assert!(error.contains("7"), "{error}");
}

#[test]
fn rtsp_restarts_after_eof_and_discards_incomplete_frames() {
    for first in ["", "printf ab;"] {
        let marker = Marker::new();
        let script = format!("if test -f \"$1\"; then printf xyz; else touch \"$1\"; {} exit 7; fi", first);
        let mut input = capture(&script, &marker, true);
        assert_eq!(input.read_frame().unwrap().unwrap().data, b"xyz");
        assert_eq!(input.stream_generation, 1);
    }
}

#[test]
fn stop_cancels_a_blocked_read_without_reconnecting() {
    let marker = Marker::new();
    let mut input = capture("exec sleep 30", &marker, true);
    let process = input.process();
    let reader = thread::spawn(move || {
        assert!(input.read_frame().unwrap().is_none());
        input.stream_generation
    });
    process.stop();
    assert_eq!(reader.join().unwrap(), 0);
    assert!(process.child.lock().unwrap().try_wait().unwrap().is_some());
}

#[test]
fn stop_during_retry_does_not_spawn_a_replacement() {
    let marker = Marker::new();
    let mut input = capture("exit 0", &marker, true);
    let process = input.process();
    let reader = thread::spawn(move || {
        assert!(input.read_frame().unwrap().is_none());
        input.stream_generation
    });
    thread::sleep(Duration::from_millis(200));
    process.stop();
    assert_eq!(reader.join().unwrap(), 0);
}

#[test]
fn rtsp_timeout_is_not_applied_to_files_or_devices() {
    for source in ["rtsp://camera/live", "rtsps://camera/live", "video.mp4", "/dev/video0"] {
        let mut command = Command::new("ffmpeg");
        configure_input(&mut command, source);
        let arguments: Vec<_> = command.get_args().map(|arg| arg.to_str().unwrap()).collect();
        if source.starts_with("rtsp") {
            assert_eq!(arguments, ["-rtsp_transport", "tcp", "-timeout", "10000000"]);
        } else {
            assert!(!arguments.contains(&"-timeout"));
        }
    }
}
