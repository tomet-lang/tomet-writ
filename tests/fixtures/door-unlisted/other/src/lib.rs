// Not on the list, and reaching for the door anyway.
pub fn peek(p: &str) -> bool {
    std::fs::metadata(p).is_ok()
}
