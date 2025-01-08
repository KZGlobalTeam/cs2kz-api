/// Returns <code>[Some](s)</code> if `s` is not empty, otherwise [`None`].
pub fn empty_as_none(s: &str) -> Option<&str> {
    match s {
        "" => None,
        non_empty => Some(non_empty),
    }
}
