use std::{fmt, io, str};

/// A compatibility shim that implements [`io::Write`] in terms of [`fmt::Write`].
pub struct IoCompat<'a, 'f>(pub &'f mut fmt::Formatter<'a>);

impl io::Write for IoCompat<'_, '_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        str::from_utf8(buf)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
            .and_then(|str| self.0.write_str(str).map_err(io::Error::other))
            .map(|()| buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        self.0.write_fmt(fmt).map_err(io::Error::other)
    }
}
