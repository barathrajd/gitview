#![cfg(all(unix, feature = "test-support"))]

use std::{
    fs::File,
    io::Read,
    os::fd::{AsRawFd, FromRawFd},
    process::{Command, Stdio},
    thread,
};

#[test]
#[allow(clippy::unnecessary_mut_passed)]
fn ratatui_restores_the_terminal_when_the_application_panics() {
    let mut master = -1;
    let mut slave = -1;
    let mut size = libc::winsize {
        ws_row: 24,
        ws_col: 80,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let result = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut size,
        )
    };
    assert_eq!(result, 0, "could not create a pseudo-terminal");

    let slave = unsafe { File::from_raw_fd(slave) };
    let before = terminal_attributes(slave.as_raw_fd());
    let mut command = Command::new(env!("CARGO_BIN_EXE_terminal_panic_probe"));
    command
        .stdin(Stdio::from(slave.try_clone().unwrap()))
        .stdout(Stdio::from(slave.try_clone().unwrap()))
        .stderr(Stdio::from(slave.try_clone().unwrap()));
    let mut child = command.spawn().unwrap();
    let mut terminal_output = unsafe { File::from_raw_fd(master) };
    let reader = thread::spawn(move || {
        let mut output = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            match terminal_output.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => output.extend_from_slice(&buffer[..count]),
                Err(error) if error.raw_os_error() == Some(libc::EIO) => {
                    break;
                }
                Err(error) => panic!("could not read pseudo-terminal output: {error}"),
            }
        }
        output
    });

    let status = child.wait().unwrap();
    drop(command);
    let after = terminal_attributes(slave.as_raw_fd());
    assert_eq!(
        after.c_lflag & (libc::ICANON | libc::ECHO),
        before.c_lflag & (libc::ICANON | libc::ECHO),
        "terminal input mode was not restored"
    );
    drop(slave);
    let output = String::from_utf8_lossy(&reader.join().unwrap()).into_owned();
    assert!(!status.success(), "the probe must panic");
    assert!(
        output.contains("\u{1b}[?1049l"),
        "alternate screen was not restored"
    );
}

fn terminal_attributes(fd: libc::c_int) -> libc::termios {
    let mut attributes = std::mem::MaybeUninit::uninit();
    let result = unsafe { libc::tcgetattr(fd, attributes.as_mut_ptr()) };
    assert_eq!(result, 0, "could not inspect pseudo-terminal mode");
    unsafe { attributes.assume_init() }
}
