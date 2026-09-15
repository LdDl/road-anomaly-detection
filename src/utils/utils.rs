use serde::Serializer;
use crate::frame::RawFrame;
use base64::{
    Engine,
    engine::general_purpose
};

pub fn serialize_mat_as_base64<S>(mat: &Option<RawFrame>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match mat {
        Some(m) => {
            let base64_string = mat_as_base64(m).map_err(serde::ser::Error::custom)?;
            serializer.serialize_str(&base64_string)
        },
        None => serializer.serialize_none()
    }
}

pub fn mat_as_base64(mat: &RawFrame) -> Result<String, png::EncodingError> {
    let mut rgb = mat.data.clone();
    for pixel in rgb.chunks_exact_mut(3) {
        pixel.swap(0, 2);
    }
    let mut buf = Vec::new();
    let mut encoder = png::Encoder::new(&mut buf, mat.width, mat.height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&rgb)?;
    writer.finish()?;
    // Convert the bytes to a base64 string
    Ok(general_purpose::STANDARD.encode(&buf))
}
