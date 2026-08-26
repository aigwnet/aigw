use std::{
    fmt::Display,
    io::Write,
    ops::{Deref, DerefMut},
};

pub struct Buffer {
    space_before: usize,
    buffer: [u8; 65535],
    start: usize,
    end: usize,
}

impl Buffer {
    ///
    /// Create new buffer
    ///
    pub fn new(space_before: usize) -> Self {
        Self {
            buffer: [0; 65535],
            space_before,
            start: space_before,
            end: space_before,
        }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn clear(&mut self) {
        self.set_start(self.space_before);
        let _ = self.set_len(0);
    }

    pub(crate) fn get_start(&self) -> usize {
        self.start
    }

    pub(crate) fn set_start(&mut self, start: usize) {
        self.start = start
    }

    pub fn set_len(&mut self, length: usize) -> std::io::Result<()> {
        let end = self.start.checked_add(length).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "buffer length overflow")
        })?;
        if end > self.buffer.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "buffer length exceeds capacity",
            ));
        }
        self.end = end;
        Ok(())
    }

    pub(crate) fn prepend_byte(&mut self, byte: u8) {
        self.start -= 1;
        self.buffer[self.start] = byte
    }

    pub fn take_prefix(&mut self) -> u8 {
        let byte = self.buffer[self.start];
        self.start += 1;
        byte
    }

    pub fn buffer(&mut self) -> &mut [u8] {
        &mut self.buffer[self.start..]
    }

    pub fn message_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[self.start..self.end]
    }
}

impl Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let len = self.len();
        let length = buf.len();
        let end = self.end.checked_add(length).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::WriteZero, "buffer length overflow")
        })?;
        if end > self.buffer.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "buffer capacity exceeded",
            ));
        }
        self.buffer[self.end..end].clone_from_slice(buf);
        self.set_len(len + length)?;
        Ok(length)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl AsRef<[u8]> for Buffer {
    fn as_ref(&self) -> &[u8] {
        &self.buffer[self.start..self.end]
    }
}

impl Deref for Buffer {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.buffer[self.start..self.end]
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[self.start..self.end]
    }
}

impl Display for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?}", &self.buffer[self.start..self.end])
    }
}
