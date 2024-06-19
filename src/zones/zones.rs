use opencv::{
    core::Mat, core::Point2f, core::Vector, core::Point2i, core::Scalar, imgproc::line, imgproc::put_text,
    imgproc::FONT_HERSHEY_SIMPLEX, imgproc::LINE_8,
    imgproc::point_polygon_test,
};

#[derive(Debug)]
pub struct Zone {
    pub id: String,
    pub color: Scalar,
    pixel_coordinates: Vector<Point2f>,
    segments: [[Point2i; 2]; 4]
}

impl Zone {
    pub fn new(id: String, coordinates: [[i32; 2]; 4], color_rgb: Option<[u16; 3]>) -> Self {
        let pixel_coordinates: Vector<Point2f> = coordinates.iter().map(|pair| {
            Point2f::new(pair[0] as f32, pair[1] as f32)
        }).collect();
        let mut segments: [[Point2i; 2]; 4] = [[Point2i::new(0, 0), Point2i::new(0, 0)], [Point2i::new(0, 0), Point2i::new(0, 0)], [Point2i::new(0, 0), Point2i::new(0, 0)], [Point2i::new(0, 0), Point2i::new(0, 0)]];
        for i in 1..coordinates.len() {
            let prev_pt = Point2i::new(
                coordinates[i - 1][0],
                coordinates[i - 1][1],
            );
            let current_pt = Point2i::new(
                coordinates[i][0],
                coordinates[i][1],
            );
            segments[i-1] = [prev_pt, current_pt];
        }
        segments[segments.len() - 1] = [Point2i::new(coordinates[coordinates.len()-1][0], coordinates[coordinates.len()-1][1]), Point2i::new(coordinates[0][0], coordinates[0][1])];
        let color = match color_rgb {
            Some(rgb_array) => Scalar::from((rgb_array[2] as f64, rgb_array[1] as f64, rgb_array[0] as f64)),
            None => Scalar::from((0., 0., 0.))
        };
        Zone{
            id: id,
            color: color,
            pixel_coordinates: pixel_coordinates,
            segments: segments
        }
    }
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        let ppt = point_polygon_test(&self.pixel_coordinates, Point2f::new(x, y), false).unwrap();
        ppt == 1.0 || ppt == 0.0 || ppt == -1.0
    }
    pub fn draw(&self, img: &mut Mat) {
        for seg in self.segments {
            line(img, seg[0], seg[1], self.color, 2, LINE_8, 0).unwrap();
        } 
    }
}
