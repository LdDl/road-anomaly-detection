# Yet another toy utility for registering anomaly situations on roads

In W.I.P. stage

## Table of Contents
- [Video showcase](#video-showcase)
- [About](#about)
- [How does it work?](#how-does-it-work?)
- [Installation and usage](#installation-and-usage)
- [Future works](#future-works)
- [References](#references)
- [Support](#support)

## Video showcase
@w.i.p.

## About
This project is aimed to register road traffic accidents, foreign object and other anomaly situations on roads. It is pretty simple and uses just YOLO (both [traditional](https://github.com/AlexeyAB/darknet) and [Ultralytics version](https://github.com/ultralytics/ultralytics) could be used). Some advanced techniques like 3D-CNN, LSTM's and others could be used, but I do not have that much time so PR's are very welcome.

OpenCV is not required. Inference uses ONNX Runtime through `od_opencv` with its OpenCV features disabled. Traditional Darknet models need to be converted to ONNX with YOLOv8 output layout before use.

I do have also utility for monitoring road traffic flow parameters [here](https://github.com/LdDl/rust-road-traffic)

## How does it work?
Diagram could speak more than a thousand words. I'm not digging into the Background Subtraction / Object Detection or MOT (Multi-object tracking) topic since it is not a main target of this project.

<img src="docs/anomaly_detection.drawio.svg" width="720">

Diagram is prepared via https://app.diagrams.net/. Source is [here](docs/anomaly_detection.drawio).

MOG2 is implemented in Rust in [src/background/background.rs](src/background/background.rs). It keeps up to five Gaussian components per pixel, adapts their weights, means and variances, removes weak components and reconstructs the background. Pixels are processed in parallel with Rayon, with reusable buffers and no allocations inside the pixel update. The detector receives the reconstructed background at the configured network resolution. History remains one second of frames, the foreground threshold is 16, and shadow detection is disabled as before.

The model update and defaults follow the [MOG2 reference implementation](https://github.com/opencv/opencv/blob/4.x/modules/video/src/bgfg_gaussmix2.cpp). This implementation handles BGR24 frames without shadow detection. Numerical equivalence and performance against the original version have not been measured.

## Screenshots

Preview window:

<img src="docs/screenshot_1.png" width="720">

Subscribe channel on the server:

<img src="docs/screenshot_2.png" width="720">

Wait until event has come:

<img src="docs/screenshot_3-1.png" height="240" width="1080">

## Installation and usage

**Important notice**: the original version was tested on Ubuntu 22.04.3 LTS. The implementation without OpenCV still needs runtime validation.

It is needed to compile it via Rust programming language compiler (here is docs: https://www.rust-lang.org/learn/get-started) currently. If further I'll make a Dockerfile for CPU atleast and may be for GPU/CUDA.

Use Rust 1.91 or newer for `od_opencv 0.8.2`. Video capture requires `ffmpeg` and `ffprobe` on `PATH`. GStreamer pipelines, including CSI cameras, also require `gst-launch-1.0`; specify frame width, height and framerate in the pipeline caps. The preview uses `minifb` and needs a desktop session; set `output.enable = false` for headless operation. Event snapshots remain PNG images encoded as Base64.

Compilation from source code:
```shell
git clone https://github.com/LdDl/road-anomaly-detection
cd road-anomaly-detection
cargo build --release
```

ONNX Runtime uses the CPU by default. For CUDA support, build with `cargo build --release --features ort-cuda` and install the CUDA/cuDNN libraries required by ONNX Runtime.

Prepare an ONNX neural network for detecting anomaly events. The supported output layout is `[1, 4 + number_of_classes, number_of_predictions]`, as used by YOLOv8/v9/v11. Convert Darknet YOLOv3/v4/v7 models with [darknet2onnx](https://github.com/LdDl/darknet2onnx) using `--format yolov8`. The legacy `network_ver` setting is optional; `network_format` defaults to `"onnx"`, and `network_cfg` is unused.

Prepare configuration file. Example could be found here - [data/conf.toml](data/conf.toml). In my example I use YOLOv8 trained on just two classes: "moderate_accident", "severe_accident".

Run:
```
export ROAD_ANOMALY_CONFIG=$(pwd)/data/conf.toml
./target/release/road-anomaly-detector $ROAD_ANOMALY_CONFIG
```

Close the preview or press Escape or S to stop. Ctrl+C also stops capture when the preview is disabled.

When events published into to the reciever server than you can expect following JSON structure:
```json
{
    "id": "Event identifier represented as UUID v4",
    "event_registered_at": UTC UnixTimestamp when event has been registered,
    "event_image": "base64 representation of an image",
    "object_id": "Detection identifier. Most of time would be represented as UUID v4",
    "object_registered_at": UTC UnixTimestamp when detection has been registered,
    "object_lifetime": Number of second while the detection was considered "relevant",
    "object_bbox": {
        "x": X-coordinate of the left top of the detection,
        "y": Y-coordinate of the left top of the detection,
        "width": Width of the corresponding detection bounding box,
        "height": Height of the corresponding detection bounding box
    },
    "object_poi": {
        "x": X-coordinate of the center of the detection bounding box,
        "y": Y-coordinate of the center of the detection bounding box
    },
    "object_classname": "Label for the class",
    "object_confidence": Confidence that detection is classified as corresponding class label,
    "zone_id": "Unique identifier for zone of interests",
    "equipment_id": "Optional application name (could be considered as equipment identifier for embedded devices)"
}
```

## Future works
* Make REST API to extract and to mutate configuration;
* Make MJPEG export;
* Make publishing to custom REST API via POST request;
* Prepare some pre-trained neural networks;

## References
* MOG2 - https://docs.opencv.org/4.x/d1/dc5/tutorial_background_subtraction.html
* MOT (Multi-object tracking) in Rust programming language - https://github.com/LdDl/mot-rs
* ONNX Runtime backend - https://docs.rs/od_opencv/0.8.2/od_opencv/
* Object detection in Rust programming language via YOLO - https://github.com/LdDl/object-detection-opencv-rust
* YOLO v3 paper - https://arxiv.org/abs/1804.02767, Joseph Redmon, Ali Farhadi
* YOLO v4 paper - https://arxiv.org/abs/2004.10934, Alexey Bochkovskiy, Chien-Yao Wang, Hong-Yuan Mark Liao
* YOLO v7 paper - https://arxiv.org/abs/2207.02696, Chien-Yao Wang, Alexey Bochkovskiy, Hong-Yuan Mark Liao
* Original Darknet YOLO repository - https://github.com/pjreddie/darknet
* Most popular fork of Darknet YOLO - https://github.com/AlexeyAB/darknet
* Developers of YOLOv8 - https://github.com/ultralytics/ultralytics. If you are aware of some original papers for YOLOv8 architecture, please contact me to mention it in this README.

Video example has been taken from [here](https://www.youtube.com/watch?v=z60Y20kJSmc)

Please cite this repository if you are using it

## Support
If you have troubles or questions please [open an issue](https://github.com/LdDl/road-anomaly-detection/issues/new).
