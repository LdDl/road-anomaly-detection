use mot_rs::mot;

#[derive(Debug)]
pub enum TrackerError {
    MOTError(mot::TrackerError),
}


impl From<mot::TrackerError> for TrackerError {
    fn from(e: mot::TrackerError) -> Self {
        TrackerError::MOTError(e)
    }
}