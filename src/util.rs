/// Turns a byte-slice into a string-slice with fallback values if the given `bytes` are either
/// empty, or invalid UTF-8.
///
/// This is intended to be used with request/response payloads.
pub const fn stringify_bytes(bytes: &[u8]) -> &str {
	match std::str::from_utf8(bytes) {
		Ok(s) if s.is_empty() => "null",
		Ok(s) => s,
		Err(_) => "<bytes>",
	}
}
