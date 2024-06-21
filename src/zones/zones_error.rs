#[derive(Debug)]
pub enum ZonesError {
    OpenCVError(opencv::Error),
}

impl From<opencv::Error> for ZonesError {
    fn from(e: opencv::Error) -> Self {
        ZonesError::OpenCVError(e)
    }
}
