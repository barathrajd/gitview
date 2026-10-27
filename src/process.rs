use std::{
    io::Read,
    process::{Command, ExitStatus, Stdio},
};

pub struct LimitedOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: ExitStatus,
    pub truncated: bool,
}

pub fn limited_output(command: &mut Command, limit: usize) -> Result<LimitedOutput, String> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not run Git: {e}"))?;
    let mut stderr = child.stderr.take().ok_or("Missing Git stderr")?;
    let error_reader = std::thread::spawn(move || {
        let mut saved = Vec::new();
        let mut buffer = [0u8; 4096];
        while let Ok(count) = stderr.read(&mut buffer) {
            if count == 0 {
                break;
            }
            let remaining = 65536usize.saturating_sub(saved.len());
            saved.extend_from_slice(&buffer[..count.min(remaining)]);
        }
        saved
    });
    let mut stdout = Vec::new();
    let mut pipe = child.stdout.take().ok_or("Missing Git stdout")?;
    let mut buffer = [0u8; 8192];
    let mut lines = 0usize;
    let mut truncated = false;
    let read_result: std::io::Result<()> = (|| {
        loop {
            let count = pipe.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            for byte in &buffer[..count] {
                if stdout.len() == limit || lines == 5000 {
                    truncated = true;
                    return Ok(());
                }
                stdout.push(*byte);
                if *byte == b'\n' {
                    lines += 1;
                }
            }
        }
        Ok(())
    })();
    drop(pipe);
    if truncated || read_result.is_err() {
        let _ = child.kill();
    }
    let status = child
        .wait()
        .map_err(|e| format!("Could not wait for Git: {e}"))?;
    let stderr = error_reader.join().unwrap_or_default();
    read_result.map_err(|e| format!("Could not read Git output: {e}"))?;
    if truncated {
        stdout.truncate(limit);
    }
    Ok(LimitedOutput {
        stdout,
        stderr,
        status,
        truncated,
    })
}
