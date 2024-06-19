use std::fmt;

#[derive(Debug)]
struct TrackerInternalError{typ: i16}
impl fmt::Display for TrackerInternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.typ {
            _ => write!(f, "Undefined Tracker error")
        }
    }
}

#[derive(Debug)]
pub enum TrackerError {
    //@ Todo
}
