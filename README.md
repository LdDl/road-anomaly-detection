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

I do have also utility for monitoring road traffic flow parameters [here](https://github.com/LdDl/rust-road-traffic)

## How does it work?
Diagram could speak more than a thousand words. I'm not digging into the Background Subtraction / Object Detection or MOT (Multi-object tracking) topic since it is not a main target of this project.

<img src="docs/anomaly_detection.drawio.svg" width="720">

Diagram is prepared via https://app.diagrams.net/. Source is [here](docs/anomaly_detection.drawio).

## Screenshots
@w.i.p.

## Installation and usage

**Important notice**: it has been tested on Ubuntu 22.04.3 LTS only!

It is needed to compile it via Rust programming language compiler (here is docs: https://www.rust-lang.org/learn/get-started) currently. If further I'll make a Dockerfile for CPU atleast and may be for GPU/CUDA.

Compilation from source code:
```shell
git clone https://github.com/LdDl/road-anomaly-detection
cd road-anomaly-detection
cargo build --release
```

Prepare neural network for detecting anomaly events.

Prepare configuration file. Example could be found here - [data/conf.toml](data/conf.toml). In my example I use YOLOv8 trained on just two classes: "moderate_accident", "severe_accident".

Run:
```
export ROAD_ANOMALY_CONFIG=$(pwd)/data/conf.toml
./target/release/road-anomaly-detector $ROAD_ANOMALY_CONFIG
```

## Future works
* Make REST API to extract and to mutate configuration;
* Make MJPEG export;
* Make publishing to custom REST API via POST request;
* Prepare some pre-trained neural networks;

## References
* MOG2 - https://docs.opencv.org/4.x/d1/dc5/tutorial_background_subtraction.html
* MOT (Multi-object tracking) in Rust programming language - https://github.com/LdDl/mot-rs
* OpenCV's bindings - https://github.com/twistedfall/opencv-rust
* Object detection in Rust programming language via YOLO - https://github.com/LdDl/object-detection-opencv-rust
* YOLO v3 paper - https://arxiv.org/abs/1804.02767, Joseph Redmon, Ali Farhadi
* YOLO v4 paper - https://arxiv.org/abs/2004.10934, Alexey Bochkovskiy, Chien-Yao Wang, Hong-Yuan Mark Liao
* YOLO v7 paper - https://arxiv.org/abs/2207.02696, Chien-Yao Wang, Alexey Bochkovskiy, Hong-Yuan Mark Liao
* Original Darknet YOLO repository - https://github.com/pjreddie/darknet
* Most popular fork of Darknet YOLO - https://github.com/AlexeyAB/darknet
* Developers of YOLOv8 - https://github.com/ultralytics/ultralytics. If you are aware of some original papers for YOLOv8 architecture, please contact me to mention it in this README.

Please cite this repository if you are using it

## Support
If you have troubles or questions please [open an issue](https://github.com/LdDl/road-anomaly-detection/issues/new).