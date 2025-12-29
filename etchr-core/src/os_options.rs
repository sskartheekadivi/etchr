#![allow(unused_imports)]
#![allow(dead_code)]
#[cfg(unix)]
pub(crate) use std::os::unix::fs::OpenOptionsExt;

#[cfg(windows)]
pub(crate) trait OpenOptionsExt {
    fn custom_flags(&mut self, flags: u32) -> &mut Self;
}

#[cfg(windows)]
impl OpenOptionsExt for std::fs::OpenOptions {
    fn custom_flags(&mut self, _flags: u32) -> &mut Self {
        // On Windows, FILE_FLAG_NO_BUFFERING is handled via `CreateFileW`
        // which is not directly exposed in `std::fs::OpenOptions`.
        // This would require using `winapi` or `windows-sys` directly
        // to open the file handle. For now, this is a placeholder.
        self
    }
}
