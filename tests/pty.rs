//! End-to-end tests of the terminal handling: the CLI runs in a pseudo-terminal whose other end
//! is a scripted fake terminal, which answers (or ignores) the queries the CLI sends. The tests
//! check what reaches the terminal: queries, the image and its placement, and error messages.

#![cfg(unix)]

use std::{
    fs::File,
    io::{self, Read, Write},
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    os::unix::{fs::PermissionsExt, process::CommandExt},
    process::{Command, ExitStatus, Stdio},
    ptr,
    time::{Duration, Instant},
};

const APC_START: &[u8] = b"\x1b_G";
const ST: &[u8] = b"\x1b\\";
const TMUX_START: &[u8] = b"\x1bPtmux;";
const DA1_REQUEST: &[u8] = b"\x1b[c";
const PIXEL_SIZE_REQUEST: &[u8] = b"\x1b[14t";
const CELL_SIZE_REQUEST: &[u8] = b"\x1b[18t";

/// Longer than every timeout in the CLI, so a test only fails this way if the CLI hangs.
const HANG_TIMEOUT: Duration = Duration::from_secs(30);

/// A window of 160x50 cells, 10x20 pixels each.
const COLS: u16 = 160;
const ROWS: u16 = 50;
const X_PIX: u16 = 1600;
const Y_PIX: u16 = 1000;

/// The plot size the CLI picks for the window above: the full width less a column, capped at
/// twice the height, which is 60% of the window.
const PLOT_SIZE: (u32, u32) = (1200, 600);

#[derive(Clone, Copy)]
struct FakeTerminal {
    /// Whether the pty reports its size in cells (`TIOCGWINSZ`).
    cells_in_winsize: bool,
    /// Whether the pty reports its size in pixels.
    pixels_in_winsize: bool,
    /// Answers the Kitty graphics query with `OK`.
    graphics: bool,
    /// Answers the `CSI 14t`/`18t` size queries.
    size_replies: bool,
    /// Answers the DA1 request.
    da1: bool,
}

impl FakeTerminal {
    /// Kitty-like: supports graphics and reports everything.
    const KITTY: FakeTerminal = FakeTerminal {
        cells_in_winsize: true,
        pixels_in_winsize: true,
        graphics: true,
        size_replies: true,
        da1: true,
    };

    /// Answers only the requests it recognizes: the part of the output that `answered` has not
    /// seen yet is scanned for requests, so replies follow the order the requests arrive in.
    fn replies(&self, output: &[u8], answered: &mut usize) -> Vec<u8> {
        let mut replies = Vec::new();
        let mut pos = *answered;
        while pos < output.len() {
            let rest = &output[pos..];
            if rest.starts_with(APC_START) {
                let Some(len) = find(rest, ST) else { break };
                if self.graphics && contains(&rest[..len], b"a=q") {
                    replies.extend_from_slice(b"\x1b_Gi=31;OK\x1b\\");
                }
                pos += len + ST.len();
            } else if rest.starts_with(PIXEL_SIZE_REQUEST) {
                if self.size_replies {
                    replies.extend_from_slice(format!("\x1b[4;{Y_PIX};{X_PIX}t").as_bytes());
                }
                pos += PIXEL_SIZE_REQUEST.len();
            } else if rest.starts_with(CELL_SIZE_REQUEST) {
                if self.size_replies {
                    replies.extend_from_slice(format!("\x1b[8;{ROWS};{COLS}t").as_bytes());
                }
                pos += CELL_SIZE_REQUEST.len();
            } else if rest.starts_with(DA1_REQUEST) {
                if self.da1 {
                    replies.extend_from_slice(b"\x1b[?62;22c");
                }
                pos += DA1_REQUEST.len();
            } else if rest.len() < 8 && rest[0] == 0x1b {
                // possibly the start of a request that hasn't fully arrived
                break;
            } else {
                pos += 1;
            }
        }
        *answered = pos;
        replies
    }
}

struct Run {
    status: ExitStatus,
    /// Everything the CLI wrote to the terminal (stdout and stderr share it).
    output: Vec<u8>,
    elapsed: Duration,
}

impl Run {
    fn text(&self) -> String {
        String::from_utf8_lossy(&strip_apc(&self.output)).into_owned()
    }
}

/// Runs the CLI with its stdin, stdout and stderr on a new pty (which becomes its controlling
/// terminal, so `/dev/tty` works) and plays `term` on the other end.
fn run(term: FakeTerminal, args: &[&str], env: &[(&str, &str)]) -> Run {
    let (master, slave) = open_pty(term);
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_termplt"));
    cmd.args(args)
        .env_remove("TMUX")
        .env("TERM", "xterm-kitty")
        .stdin(Stdio::from(slave.try_clone().unwrap()))
        .stdout(Stdio::from(slave.try_clone().unwrap()))
        .stderr(Stdio::from(slave));
    for (key, value) in env {
        cmd.env(key, value);
    }
    // SAFETY: only async-signal-safe calls between fork and exec
    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() < 0 || libc::ioctl(0, libc::TIOCSCTTY as _, 0) < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let start = Instant::now();
    let mut child = cmd.spawn().unwrap();
    // drop our copies of the slave (moved into `cmd`) so reads fail once the CLI exits
    drop(cmd);

    let mut master = File::from(master);
    let mut output = Vec::new();
    let mut answered = 0;
    loop {
        assert!(
            start.elapsed() < HANG_TIMEOUT,
            "the CLI hung; output so far: {:?}",
            String::from_utf8_lossy(&strip_apc(&output))
        );
        if !poll_readable(&master, Duration::from_millis(100)) {
            if child.try_wait().unwrap().is_some() && !poll_readable(&master, Duration::ZERO) {
                break;
            }
            continue;
        }
        let mut buf = [0u8; 1 << 16];
        match master.read(&mut buf) {
            // Linux reports EIO once the last slave descriptor is closed
            Ok(0) | Err(_) => break,
            Ok(n) => output.extend_from_slice(&buf[..n]),
        }
        let replies = term.replies(&output, &mut answered);
        if !replies.is_empty() {
            master.write_all(&replies).unwrap();
        }
    }
    let status = child.wait().unwrap();
    Run {
        status,
        output,
        elapsed: start.elapsed(),
    }
}

fn open_pty(term: FakeTerminal) -> (OwnedFd, OwnedFd) {
    let cells = |n| if term.cells_in_winsize { n } else { 0 };
    let pixels = |n| if term.pixels_in_winsize { n } else { 0 };
    let mut winsize = libc::winsize {
        ws_row: cells(ROWS),
        ws_col: cells(COLS),
        ws_xpixel: pixels(X_PIX),
        ws_ypixel: pixels(Y_PIX),
    };
    let (mut master, mut slave) = (-1, -1);
    // SAFETY: valid out-pointers; a null name and termios are allowed
    let rc = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            ptr::null_mut(),
            ptr::null_mut(),
            // `*mut` on macOS, `*const` on Linux
            &raw mut winsize,
        )
    };
    assert_eq!(rc, 0, "openpty: {}", io::Error::last_os_error());
    // SAFETY: openpty returned two new descriptors that nothing else owns
    unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) }
}

fn poll_readable(file: &File, timeout: Duration) -> bool {
    let mut pollfd = libc::pollfd {
        fd: file.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: one valid pollfd
    let rc = unsafe { libc::poll(&mut pollfd, 1, timeout.as_millis() as libc::c_int) };
    rc > 0
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    find(haystack, needle).is_some()
}

fn count(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|w| *w == needle)
        .count()
}

/// Graphics commands as (control data, payload) pairs.
fn apc_commands(output: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut commands = Vec::new();
    let mut rest = output;
    while let Some(start) = find(rest, APC_START) {
        rest = &rest[start + APC_START.len()..];
        let end = find(rest, ST).expect("unterminated graphics command");
        let body = &rest[..end];
        let sep = body.iter().position(|&b| b == b';').unwrap_or(body.len());
        commands.push((
            String::from_utf8(body[..sep].to_vec()).unwrap(),
            body.get(sep + 1..).unwrap_or_default().to_vec(),
        ));
        rest = &rest[end + ST.len()..];
    }
    commands
}

/// Replaces graphics commands with `<image>` so the rest of the output can be read as text.
fn strip_apc(output: &[u8]) -> Vec<u8> {
    let mut text = Vec::new();
    let mut rest = output;
    while let Some(start) = find(rest, APC_START) {
        text.extend_from_slice(&rest[..start]);
        match find(&rest[start..], ST) {
            Some(end) => {
                text.extend_from_slice(b"<image>");
                rest = &rest[start + end + ST.len()..];
            }
            None => {
                rest = &rest[start..];
                break;
            }
        }
    }
    text.extend_from_slice(rest);
    text
}

/// What the CLI wrote after the last graphics command.
fn after_last_image(output: &[u8]) -> &[u8] {
    let end = output
        .windows(ST.len())
        .rposition(|w| w == ST)
        .expect("no graphics command");
    &output[end + ST.len()..]
}

/// Undoes tmux DCS passthrough: the wrapper is removed and doubled escapes are halved.
fn unwrap_tmux(output: &[u8]) -> Vec<u8> {
    let mut plain = Vec::new();
    let mut i = 0;
    while i < output.len() {
        if output[i..].starts_with(TMUX_START) {
            i += TMUX_START.len();
            while i < output.len() {
                match (output[i], output.get(i + 1)) {
                    (0x1b, Some(0x1b)) => {
                        plain.push(0x1b);
                        i += 2;
                    }
                    (0x1b, Some(b'\\')) => {
                        i += 2;
                        break;
                    }
                    // tmux would end the passthrough here and drop the rest
                    (0x1b, _) => panic!("undoubled ESC inside tmux passthrough at byte {i}"),
                    (b, _) => {
                        plain.push(b);
                        i += 1;
                    }
                }
            }
        } else {
            plain.push(output[i]);
            i += 1;
        }
    }
    plain
}

fn base64_decode(data: &[u8]) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut bytes = Vec::with_capacity(data.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for &c in data.iter().filter(|&&c| c != b'=') {
        let value = ALPHABET
            .iter()
            .position(|&a| a == c)
            .expect("invalid base64") as u32;
        acc = (acc << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push((acc >> bits) as u8);
        }
    }
    bytes
}

/// Checks that exactly one image was sent, as a PNG displayed without replies, and returns its
/// size.
fn sent_image(output: &[u8]) -> (u32, u32) {
    let commands: Vec<_> = apc_commands(output)
        .into_iter()
        .filter(|(ctrl, _)| !ctrl.contains("a=q"))
        .collect();
    assert!(!commands.is_empty(), "no image was sent");
    let first = &commands[0].0;
    for key in ["a=T", "f=100", "q=2"] {
        assert!(
            first.split(',').any(|kv| kv == key),
            "{key} missing: {first}"
        );
    }
    // one image: only the last chunk ends the transmission
    let ends = commands
        .iter()
        .filter(|(ctrl, _)| ctrl.contains("m=0"))
        .count();
    assert_eq!(ends, 1, "expected a single image");
    let payload: Vec<u8> = commands.iter().flat_map(|(_, p)| p.clone()).collect();
    let png =
        image::load_from_memory_with_format(&base64_decode(&payload), image::ImageFormat::Png)
            .expect("the payload is not a PNG");
    (png.width(), png.height())
}

const DATA: [&str; 2] = ["--data", "(0,0),(1,1),(2,4),(3,9)"];

#[test]
fn draws_in_a_kitty_terminal() {
    let run = run(FakeTerminal::KITTY, &DATA, &[]);
    assert!(run.status.success(), "{}", run.text());
    assert_eq!(sent_image(&run.output), PLOT_SIZE);
    // the size came from the pty, so only the graphics query and its sentinel were sent
    assert_eq!(count(&run.output, b"a=q"), 1);
    assert_eq!(count(&run.output, DA1_REQUEST), 1);
    assert!(!contains(&run.output, PIXEL_SIZE_REQUEST));
    // the cursor ends up on the line below the image
    assert_eq!(after_last_image(&run.output), b"\r\n");
}

#[test]
fn verbose_reports_the_checks() {
    let run = run(FakeTerminal::KITTY, &[DATA[0], DATA[1], "--verbose"], &[]);
    assert!(run.status.success(), "{}", run.text());
    let text = run.text();
    assert!(
        text.contains("supports the Kitty graphics protocol"),
        "{text}"
    );
    assert!(text.contains("160x50 cells, 1600x1000 pixels"), "{text}");
}

#[test]
fn asks_for_the_pixel_size_when_the_pty_has_none() {
    let term = FakeTerminal {
        pixels_in_winsize: false,
        ..FakeTerminal::KITTY
    };
    let run = run(term, &DATA, &[]);
    assert!(run.status.success(), "{}", run.text());
    assert!(contains(&run.output, PIXEL_SIZE_REQUEST));
    assert_eq!(sent_image(&run.output), PLOT_SIZE);
}

#[test]
fn estimates_the_size_when_the_terminal_does_not_report_it() {
    let term = FakeTerminal {
        pixels_in_winsize: false,
        size_replies: false,
        ..FakeTerminal::KITTY
    };
    let run = run(term, &DATA, &[]);
    assert!(run.status.success(), "{}", run.text());
    assert!(run.text().contains("did not report its size in pixels"));
    // 160x50 cells of the assumed 9x18 pixels
    assert_eq!(sent_image(&run.output), (1080, 540));
}

#[test]
fn fails_fast_without_graphics_support() {
    let term = FakeTerminal {
        graphics: false,
        ..FakeTerminal::KITTY
    };
    let run = run(term, &DATA, &[]);
    assert!(!run.status.success());
    let text = run.text();
    assert!(
        text.contains("does not support the Kitty graphics protocol"),
        "{text}"
    );
    assert!(text.contains("--output"), "{text}");
    // only the query was sent, and the DA1 reply ended the wait long before the timeout
    assert_eq!(apc_commands(&run.output).len(), 1);
    // (the query timeout is 2 s)
    assert!(
        run.elapsed < Duration::from_millis(1500),
        "{:?}",
        run.elapsed
    );
}

#[test]
fn draws_anyway_when_the_terminal_answers_nothing() {
    let term = FakeTerminal {
        graphics: false,
        size_replies: false,
        da1: false,
        ..FakeTerminal::KITTY
    };
    let run = run(term, &DATA, &[]);
    assert!(run.status.success(), "{}", run.text());
    assert!(
        run.text()
            .contains("did not answer a graphics support query")
    );
    assert_eq!(sent_image(&run.output), PLOT_SIZE);
}

#[test]
fn assumes_a_standard_window_when_nothing_reports_a_size() {
    let term = FakeTerminal {
        cells_in_winsize: false,
        pixels_in_winsize: false,
        graphics: false,
        size_replies: false,
        da1: false,
    };
    let run = run(
        term,
        &[DATA[0], DATA[1], "--width", "400", "--height", "300"],
        &[],
    );
    assert!(run.status.success(), "{}", run.text());
    let text = run.text();
    assert!(
        text.contains("did not report its size; assuming 80x24 cells"),
        "{text}"
    );
    // no message may claim a query was for graphics when it was for the size
    assert!(!text.contains("is required"), "{text}");
    assert_eq!(sent_image(&run.output), (400, 300));
}

#[test]
fn explicit_size_is_used() {
    let run = run(
        FakeTerminal::KITTY,
        &[DATA[0], DATA[1], "--width", "400", "--height", "300"],
        &[],
    );
    assert!(run.status.success(), "{}", run.text());
    assert_eq!(sent_image(&run.output), (400, 300));
}

/// A `tmux` on `PATH` that reports `allow-passthrough` as `value`.
fn fake_tmux(value: &str) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("tmux");
    std::fs::write(&script, format!("#!/bin/sh\necho {value}\n")).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        dir.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    (dir, path)
}

#[test]
fn tmux_wraps_the_image_and_moves_the_cursor_itself() {
    let (_dir, path) = fake_tmux("on");
    let env = [("TMUX", "/tmp/tmux-1000/default,1,0"), ("PATH", &path)];
    let run = run(FakeTerminal::KITTY, &DATA, &env);
    assert!(run.status.success(), "{}", run.text());
    // the outer terminal's reply wouldn't reach the CLI, so nothing is queried
    assert!(!contains(&run.output, DA1_REQUEST));
    let plain = unwrap_tmux(&run.output);
    assert!(!contains(&plain, b"a=q"));
    // every graphics command is wrapped
    assert_eq!(
        count(&run.output, TMUX_START),
        apc_commands(&plain).len(),
        "unwrapped graphics commands"
    );
    assert_eq!(sent_image(&plain), PLOT_SIZE);
    let first = &apc_commands(&plain)[0].0;
    assert!(first.split(',').any(|kv| kv == "C=1"), "{first}");
    // 600 pixels over 20-pixel rows
    assert_eq!(after_last_image(&plain), "\r\n".repeat(30).as_bytes());
}

#[test]
fn tmux_without_passthrough_is_an_error() {
    let (_dir, path) = fake_tmux("off");
    let env = [("TMUX", "/tmp/tmux-1000/default,1,0"), ("PATH", &path)];
    let run = run(FakeTerminal::KITTY, &DATA, &env);
    assert!(!run.status.success());
    assert!(
        run.text().contains("allow-passthrough on"),
        "{}",
        run.text()
    );
    assert!(!contains(&run.output, APC_START));
}
