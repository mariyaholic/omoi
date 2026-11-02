//! Browser openings cannot be reconstructed into measured playback sessions.
pub fn available() -> bool {
    false
}
pub fn run() -> crate::Result<()> {
    Err(crate::Error::NotImplemented(
        "playback sessions require an opt-in listener",
    ))
}
