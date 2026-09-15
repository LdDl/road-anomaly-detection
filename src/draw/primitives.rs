use crate::draw::Scalar;
use crate::frame::RawFrame;

fn set_pixel(img: &mut RawFrame, x: i32, y: i32, color: Scalar) {
    if x >= 0 && y >= 0 && x < img.width as i32 && y < img.height as i32 {
        let offset = (y as usize * img.width as usize + x as usize) * 3;
        img.data[offset..offset + 3].copy_from_slice(&color.0);
    }
}

pub fn draw_line(img: &mut RawFrame, from: [i32; 2], to: [i32; 2], color: Scalar, thickness: i32) {
    if img.width == 0 || img.height == 0 {
        return;
    }
    let dx = to[0] as f64 - from[0] as f64;
    let dy = to[1] as f64 - from[1] as f64;
    let mut start: f64 = 0.0;
    let mut end: f64 = 1.0;
    // Clip the segment before drawing to bound the work by the image dimensions.
    for (direction, distance) in [(-dx, from[0] as f64), (dx, img.width as f64 - 1.0 - from[0] as f64), (-dy, from[1] as f64), (dy, img.height as f64 - 1.0 - from[1] as f64)] {
        if direction == 0.0 {
            if distance < 0.0 {
                return;
            }
        } else if direction < 0.0 {
            start = start.max(distance / direction);
        } else {
            end = end.min(distance / direction);
        }
    }
    if start > end {
        return;
    }
    let mut x = (from[0] as f64 + start * dx).round() as i32;
    let mut y = (from[1] as f64 + start * dy).round() as i32;
    let end_x = (from[0] as f64 + end * dx).round() as i32;
    let end_y = (from[1] as f64 + end * dy).round() as i32;
    let dx = (end_x as i64 - x as i64).abs();
    let dy = -(end_y as i64 - y as i64).abs();
    let step_x = if x < end_x { 1 } else { -1 };
    let step_y = if y < end_y { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        for offset in 0..thickness.max(1) {
            if dx >= -dy {
                set_pixel(img, x, y + offset, color);
            } else {
                set_pixel(img, x + offset, y, color);
            }
        }
        if x == end_x && y == end_y {
            break;
        }
        let double_error = 2 * error;
        if double_error >= dy {
            error += dy;
            x += step_x;
        }
        if double_error <= dx {
            error += dx;
            y += step_y;
        }
    }
}

pub fn draw_text(img: &mut RawFrame, anchor: [i32; 2], text: &str, color: Scalar) {
    for (index, character) in text.chars().enumerate() {
        let glyph = match character {
            '0' => [0b0110, 0b1001, 0b1001, 0b1001, 0b0110],
            '1' => [0b0010, 0b0110, 0b0010, 0b0010, 0b0111],
            '2' => [0b0110, 0b1001, 0b0010, 0b0100, 0b1111],
            '3' => [0b0110, 0b1001, 0b0010, 0b1001, 0b0110],
            '4' => [0b1010, 0b1010, 0b1111, 0b0010, 0b0010],
            '5' => [0b1111, 0b1000, 0b1110, 0b0001, 0b1110],
            '6' => [0b0110, 0b1000, 0b1110, 0b1001, 0b0110],
            '7' => [0b1111, 0b0001, 0b0010, 0b0100, 0b0100],
            '8' => [0b0110, 0b1001, 0b0110, 0b1001, 0b0110],
            '9' => [0b0110, 0b1001, 0b0111, 0b0001, 0b0110],
            'a' => [0b0110, 0b1001, 0b1111, 0b1001, 0b1001],
            'b' => [0b1110, 0b1001, 0b1110, 0b1001, 0b1110],
            'c' => [0b0110, 0b1001, 0b1000, 0b1001, 0b0110],
            'd' => [0b1110, 0b1001, 0b1001, 0b1001, 0b1110],
            'e' => [0b1111, 0b1000, 0b1110, 0b1000, 0b1111],
            'f' => [0b1111, 0b1000, 0b1110, 0b1000, 0b1000],
            '-' => [0b0000, 0b0000, 0b1111, 0b0000, 0b0000],
            _ => [0; 5],
        };
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..4 {
                if bits & (1 << (3 - column)) != 0 {
                    for y in 0..2 {
                        for x in 0..2 {
                            set_pixel(img, anchor[0] + index as i32 * 10 + column * 2 + x, anchor[1] + row as i32 * 2 + y, color);
                        }
                    }
                }
            }
        }
    }
}
