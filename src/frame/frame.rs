use image::{RgbImage, imageops::{resize, FilterType}};
use od_opencv::ImageBuffer;

// BGR24 pixels stored row by row without padding, as in rust-road-traffic.
#[derive(Debug, Clone, Default)]
pub struct RawFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl RawFrame {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            data: vec![0; width as usize * height as usize * 3],
            width,
            height,
        }
    }

    pub fn resize(&self, width: u32, height: u32) -> Self {
        // Resizing treats all three channels equally, so BGR order is preserved.
        let img = RgbImage::from_raw(self.width, self.height, self.data.clone()).expect("Invalid frame dimensions");
        let resized = resize(&img, width, height, FilterType::Triangle);
        Self {
            data: resized.into_raw(),
            width,
            height,
        }
    }

    pub fn to_image_buffer(&self) -> ImageBuffer {
        let data = ndarray::Array3::from_shape_vec((self.height as usize, self.width as usize, 3), self.data.clone()).expect("Invalid frame dimensions");
        ImageBuffer::from_bgr(data)
    }

    pub fn to_window_buffer(&self) -> Vec<u32> {
        self.data.chunks_exact(3).map(|pixel| {
            (pixel[2] as u32) << 16 | (pixel[1] as u32) << 8 | pixel[0] as u32
        }).collect()
    }
}
