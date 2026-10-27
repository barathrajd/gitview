use std::{ffi::OsString, path::PathBuf};

/// Git prints one terminating newline; preserve all bytes belonging to the path.
pub fn parse_root(output: &[u8]) -> PathBuf {
    let path = output.strip_suffix(b"\n").unwrap_or(output);
    #[cfg(unix)]
    let value = {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(path.to_vec())
    };
    #[cfg(not(unix))]
    let value = OsString::from(String::from_utf8_lossy(path).into_owned());
    PathBuf::from(value)
}
