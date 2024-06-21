use std::fmt;

#[derive(Debug)]
struct TrackerInternalError;
impl fmt::Display for TrackerInternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Undefined Tracker error")
    }
}

#[derive(Debug)]
pub enum TrackerError {
    //@ Todo
}
