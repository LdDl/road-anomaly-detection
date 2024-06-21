use serde::Serializer;
use opencv::{
    core::Mat,
    core::Vector,
    imgcodecs::imencode
};
use base64::{
    Engine,
    engine::general_purpose
};

pub fn serialize_mat_as_base64<S>(mat: &Option<Mat>, serializer: S) -> Result<S::Ok, S::Error>
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

pub fn mat_as_base64(mat: &Mat) -> Result<String, opencv::Error> {
    let mut buf = Vector::new();
    imencode(".png", mat, &mut buf, &Vector::new())?;
    // Convert the bytes to a base64 string
    Ok(general_purpose::STANDARD.encode(&buf))
}
