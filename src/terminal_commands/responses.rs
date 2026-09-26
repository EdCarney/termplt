use crate::common::Result;
use crossterm::terminal;
use std::{
    error::Error,
    fmt,
    io::{self, Write},
    time::{Duration, Instant},
};

/// How long to wait for a terminal to answer a query before giving up.
pub const DEFAULT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(2);

/// Primary Device Attributes (DA1) request. Every VT100-compatible terminal answers it, so it is
/// sent after each query as a sentinel: terminals answer requests in order, so if the DA1 reply
/// arrives without a reply to the query before it, the query is unsupported and there is no need
/// to wait for the timeout.
const DA1_REQUEST: &[u8] = b"\x1b[c";

const ESC: u8 = 0x1b;

/// Failures talking to the terminal.
#[derive(Debug)]
pub enum TerminalCommandError {
    /// No terminal is attached to send the query to (e.g. running from a pipe, cron or CI).
    NoTerminal(io::Error),
    /// The terminal answered the sentinel but not the query, so it does not support the query.
    Unsupported,
    /// The terminal did not answer within the timeout.
    Timeout(Duration),
    /// The terminal answered with something that could not be interpreted.
    InvalidResponse(String),
}

impl fmt::Display for TerminalCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TerminalCommandError::NoTerminal(e) => {
                write!(f, "No terminal available to query ({e})")
            }
            TerminalCommandError::Unsupported => write!(
                f,
                "The terminal does not support the required query; a terminal implementing the \
                 Kitty graphics protocol (e.g. Kitty, WezTerm, Ghostty) is required"
            ),
            TerminalCommandError::Timeout(timeout) => write!(
                f,
                "The terminal did not respond within {} ms; a terminal implementing the Kitty \
                 graphics protocol (e.g. Kitty, WezTerm, Ghostty) is required",
                timeout.as_millis()
            ),
            TerminalCommandError::InvalidResponse(resp) => {
                write!(f, "Unexpected response from terminal: {resp}")
            }
        }
    }
}

impl Error for TerminalCommandError {}

pub trait TermCommand {
    fn get_request(&self) -> &[u8];

    fn get_response_start(&self) -> &[u8] {
        "".as_bytes()
    }

    fn get_response_end(&self) -> &[u8] {
        "".as_bytes()
    }

    fn execute(&self) -> Result<()> {
        let mut stdout = io::stdout().lock();
        stdout.write_all(self.get_request())?;
        stdout.flush()?;
        Ok(())
    }

    /// Sends the request to the terminal and returns the body of its reply (the bytes between
    /// the response start and end markers). Waits at most [`DEFAULT_RESPONSE_TIMEOUT`].
    fn execute_with_response(&self) -> Result<String> {
        self.execute_with_response_timeout(DEFAULT_RESPONSE_TIMEOUT)
    }

    /// Like [`TermCommand::execute_with_response`], with an explicit timeout.
    fn execute_with_response_timeout(&self, timeout: Duration) -> Result<String> {
        let mut tty = sys::Tty::open().map_err(TerminalCommandError::NoTerminal)?;

        // raw mode must be active before the request is written so the reply is neither echoed
        // nor line-buffered; the guard restores the previous mode on every exit path
        let _raw_mode = RawModeGuard::enable()?;

        tty.write_all(self.get_request())?;
        tty.write_all(DA1_REQUEST)?;
        tty.flush()?;

        let resp = read_response(
            &mut tty,
            self.get_response_start(),
            self.get_response_end(),
            timeout,
        )?;
        Ok(resp)
    }
}

/// Enables terminal raw mode and restores the previous mode when dropped (including on early
/// returns and panics). Leaves raw mode alone if the caller had already enabled it.
struct RawModeGuard {
    was_enabled: bool,
}

impl RawModeGuard {
    fn enable() -> io::Result<RawModeGuard> {
        let was_enabled = terminal::is_raw_mode_enabled()?;
        if !was_enabled {
            terminal::enable_raw_mode()?;
        }
        Ok(RawModeGuard { was_enabled })
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if !self.was_enabled {
            let _ = terminal::disable_raw_mode();
        }
    }
}

/// A source of terminal input that can be read with a timeout.
pub(crate) trait ByteSource {
    /// Waits at most `timeout` for input. Returns `None` on timeout and an empty vector at end of
    /// input.
    fn read_timeout(&mut self, timeout: Duration) -> io::Result<Option<Vec<u8>>>;
}

/// Reads terminal input until the DA1 sentinel reply arrives (or the timeout expires) and
/// extracts the reply delimited by `start`/`end` that precedes it.
pub(crate) fn read_response<S: ByteSource>(
    src: &mut S,
    start: &[u8],
    end: &[u8],
    timeout: Duration,
) -> std::result::Result<String, TerminalCommandError> {
    let deadline = Instant::now() + timeout;
    let mut buf = Vec::new();

    loop {
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        match src
            .read_timeout(deadline - now)
            .map_err(TerminalCommandError::NoTerminal)?
        {
            None => break,
            Some(bytes) if bytes.is_empty() => {
                return Err(TerminalCommandError::NoTerminal(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "terminal input closed",
                )));
            }
            Some(bytes) => buf.extend_from_slice(&bytes),
        }

        if let Some(da1_start) = find_da1_reply(&buf) {
            // terminals answer in order, so any reply to the query precedes the DA1 reply
            return find_response(&buf[..da1_start], start, end)
                .ok_or(TerminalCommandError::Unsupported);
        }
    }

    // some terminals may answer the query but not DA1; accept a reply if one arrived
    find_response(&buf, start, end).ok_or(TerminalCommandError::Timeout(timeout))
}

/// Finds the first reply that begins with `start`, ends with `end` and contains no escape
/// characters in between (which would indicate it spans multiple sequences, e.g. a key press
/// followed by the real reply). Returns the bytes between the markers.
fn find_response(buf: &[u8], start: &[u8], end: &[u8]) -> Option<String> {
    let mut pos = 0;
    while let Some(offset) = find_subslice(&buf[pos..], start) {
        let body_start = pos + offset + start.len();
        if let Some(body_len) = find_subslice(&buf[body_start..], end) {
            let body = &buf[body_start..body_start + body_len];
            if !body.contains(&ESC)
                && let Ok(body) = std::str::from_utf8(body)
            {
                return Some(body.to_string());
            }
        }
        pos += offset + 1;
    }
    None
}

/// Finds a DA1 reply (`ESC [ ? <digits and semicolons> c`) and returns its starting index.
fn find_da1_reply(buf: &[u8]) -> Option<usize> {
    let mut pos = 0;
    while let Some(offset) = find_subslice(&buf[pos..], b"\x1b[?") {
        let start = pos + offset;
        let params = &buf[start + 3..];
        let params_len = params
            .iter()
            .take_while(|b| b.is_ascii_digit() || **b == b';')
            .count();
        if params.get(params_len) == Some(&b'c') {
            return Some(start);
        }
        pos = start + 1;
    }
    None
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(unix)]
mod sys {
    use super::ByteSource;
    use std::{
        fs::{File, OpenOptions},
        io::{self, Read, Write},
        os::fd::{AsRawFd, RawFd},
        time::{Duration, Instant},
    };

    /// The controlling terminal, opened directly so that queries work even when stdin/stdout are
    /// redirected.
    pub struct Tty {
        file: File,
    }

    impl Tty {
        pub fn open() -> io::Result<Tty> {
            let file = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
            Ok(Tty { file })
        }
    }

    impl Write for Tty {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.file.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.file.flush()
        }
    }

    impl ByteSource for Tty {
        fn read_timeout(&mut self, timeout: Duration) -> io::Result<Option<Vec<u8>>> {
            let deadline = Instant::now() + timeout;
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                match wait_readable(self.file.as_raw_fd(), remaining) {
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                    Err(err) => return Err(err),
                    Ok(false) => return Ok(None),
                    Ok(true) => {
                        let mut buf = [0u8; 256];
                        let n = self.file.read(&mut buf)?;
                        return Ok(Some(buf[..n].to_vec()));
                    }
                }
            }
        }
    }

    /// Waits at most `timeout` for `fd` to become readable; `Ok(false)` on timeout.
    ///
    /// macOS's `poll` does not support terminal devices: it reports `/dev/tty` as ready
    /// (`POLLNVAL`) at once, and the read that follows blocks until the terminal sends
    /// something, i.e. forever if it never answers. `select` works there.
    fn wait_readable(fd: RawFd, timeout: Duration) -> io::Result<bool> {
        if cfg!(target_os = "macos") {
            wait_select(fd, timeout)
        } else {
            wait_poll(fd, timeout)
        }
    }

    fn wait_poll(fd: RawFd, timeout: Duration) -> io::Result<bool> {
        let timeout_ms = timeout.as_millis().min(i32::MAX as u128) as libc::c_int;
        let mut pollfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: `pollfd` is a valid, initialized pollfd and the count is 1
        let rc = unsafe { libc::poll(&mut pollfd, 1, timeout_ms) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        if rc == 0 {
            return Ok(false);
        }
        // reading after POLLNVAL or POLLERR could block, so report those instead
        if pollfd.revents & (libc::POLLNVAL | libc::POLLERR) != 0 {
            return Err(io::Error::other(format!(
                "the terminal cannot be polled (revents {:#x})",
                pollfd.revents
            )));
        }
        Ok(true)
    }

    fn wait_select(fd: RawFd, timeout: Duration) -> io::Result<bool> {
        if fd < 0 || fd as usize >= libc::FD_SETSIZE as usize {
            return Err(io::Error::other(format!(
                "file descriptor {fd} is out of range for select"
            )));
        }
        // SAFETY: an all-zero fd_set is valid, and fd is within FD_SETSIZE (checked above)
        let mut read_fds: libc::fd_set = unsafe { std::mem::zeroed() };
        unsafe {
            libc::FD_ZERO(&mut read_fds);
            libc::FD_SET(fd, &mut read_fds);
        }
        let mut tv = libc::timeval {
            tv_sec: timeout.as_secs().min(i32::MAX as u64) as libc::time_t,
            tv_usec: timeout.subsec_micros() as _,
        };
        // SAFETY: valid fd_set and timeval; the unused sets may be null
        let rc = unsafe {
            libc::select(
                fd + 1,
                &mut read_fds,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut tv,
            )
        };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(rc > 0)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::{io::Write, os::unix::net::UnixStream};

        #[test]
        fn both_waits_time_out_and_see_data() {
            for wait in [wait_poll, wait_select] {
                let (mut tx, rx) = UnixStream::pair().unwrap();
                let fd = rx.as_raw_fd();
                let start = Instant::now();
                assert!(!wait(fd, Duration::from_millis(50)).unwrap());
                assert!(start.elapsed() >= Duration::from_millis(40));
                tx.write_all(b"x").unwrap();
                assert!(wait(fd, Duration::from_secs(5)).unwrap());
            }
        }

        #[test]
        fn polling_a_closed_descriptor_is_an_error_not_a_read() {
            let fd = UnixStream::pair().unwrap().0.as_raw_fd();
            // the stream above is already closed, so fd is invalid (POLLNVAL)
            assert!(wait_poll(fd, Duration::from_millis(10)).is_err());
        }
    }
}

#[cfg(not(unix))]
mod sys {
    use super::ByteSource;
    use std::{
        fs::{File, OpenOptions},
        io::{self, Read, Write},
        sync::{
            Mutex, OnceLock,
            mpsc::{self, Receiver, RecvTimeoutError},
        },
        thread,
        time::Duration,
    };

    /// The console's input and output buffers. Opened by name rather than through stdin/stdout
    /// so queries still work when those are redirected (e.g. data piped into the CLI).
    const CONSOLE_IN: &str = "CONIN$";
    const CONSOLE_OUT: &str = "CONOUT$";

    /// Console input cannot be polled with a timeout using only the standard library, so a
    /// background thread performs blocking reads from the console and forwards the bytes over a
    /// channel. The thread is shared by all queries and lives for the rest of the process.
    static INPUT: OnceLock<Mutex<Receiver<io::Result<Vec<u8>>>>> = OnceLock::new();

    fn input() -> &'static Mutex<Receiver<io::Result<Vec<u8>>>> {
        INPUT.get_or_init(|| {
            let (tx, rx) = mpsc::channel();
            thread::spawn(move || {
                let mut console = match File::open(CONSOLE_IN) {
                    Ok(console) => console,
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        return;
                    }
                };
                let mut buf = [0u8; 256];
                loop {
                    let res = console.read(&mut buf).map(|n| buf[..n].to_vec());
                    let done = !matches!(&res, Ok(bytes) if !bytes.is_empty());
                    if tx.send(res).is_err() || done {
                        break;
                    }
                }
            });
            Mutex::new(rx)
        })
    }

    pub struct Tty {
        out: File,
    }

    impl Tty {
        pub fn open() -> io::Result<Tty> {
            // fails when the process has no console (e.g. running as a service)
            File::open(CONSOLE_IN)?;
            let out = OpenOptions::new().write(true).open(CONSOLE_OUT)?;
            Ok(Tty { out })
        }
    }

    impl Write for Tty {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.out.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.out.flush()
        }
    }

    impl ByteSource for Tty {
        fn read_timeout(&mut self, timeout: Duration) -> io::Result<Option<Vec<u8>>> {
            let rx = input()
                .lock()
                .map_err(|_| io::Error::other("terminal input reader poisoned"))?;
            match rx.recv_timeout(timeout) {
                Ok(res) => res.map(Some),
                Err(RecvTimeoutError::Timeout) => Ok(None),
                Err(RecvTimeoutError::Disconnected) => Ok(Some(Vec::new())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// Replays scripted chunks; an exhausted script behaves like a silent terminal.
    struct FakeTerminal {
        chunks: VecDeque<Vec<u8>>,
    }

    impl FakeTerminal {
        fn new(chunks: &[&[u8]]) -> FakeTerminal {
            FakeTerminal {
                chunks: chunks.iter().map(|c| c.to_vec()).collect(),
            }
        }
    }

    impl ByteSource for FakeTerminal {
        fn read_timeout(&mut self, _timeout: Duration) -> io::Result<Option<Vec<u8>>> {
            Ok(self.chunks.pop_front())
        }
    }

    const TIMEOUT: Duration = Duration::from_millis(50);

    fn read(chunks: &[&[u8]]) -> std::result::Result<String, TerminalCommandError> {
        read_response(&mut FakeTerminal::new(chunks), b"\x1b[", b"t", TIMEOUT)
    }

    #[test]
    fn reply_followed_by_sentinel() {
        assert_eq!(
            read(&[b"\x1b[4;600;800t\x1b[?62;22c"]).unwrap(),
            "4;600;800"
        );
    }

    #[test]
    fn reply_split_across_reads() {
        assert_eq!(
            read(&[b"\x1b[4;6", b"00;800t", b"\x1b[?6", b"2c"]).unwrap(),
            "4;600;800"
        );
    }

    #[test]
    fn sentinel_without_reply_is_unsupported() {
        assert!(matches!(
            read(&[b"\x1b[?62;22c"]),
            Err(TerminalCommandError::Unsupported)
        ));
    }

    #[test]
    fn silent_terminal_times_out() {
        assert!(matches!(read(&[]), Err(TerminalCommandError::Timeout(_))));
    }

    #[test]
    fn reply_without_sentinel_is_accepted_at_timeout() {
        assert_eq!(read(&[b"\x1b[4;600;800t"]).unwrap(), "4;600;800");
    }

    #[test]
    fn key_press_before_reply_is_ignored() {
        // an arrow key (ESC [ A) arrives before the reply
        assert_eq!(
            read(&[b"\x1b[A\x1b[4;600;800t\x1b[?1;2c"]).unwrap(),
            "4;600;800"
        );
    }

    #[test]
    fn end_of_input_is_an_error() {
        assert!(matches!(
            read(&[b""]),
            Err(TerminalCommandError::NoTerminal(_))
        ));
    }

    #[test]
    fn kitty_style_reply() {
        let resp = read_response(
            &mut FakeTerminal::new(&[b"\x1b_Gi=31;OK\x1b\\\x1b[?62c"]),
            b"\x1b_G",
            b"\x1b\\",
            TIMEOUT,
        );
        assert_eq!(resp.unwrap(), "i=31;OK");
    }

    #[test]
    fn da1_detection_requires_terminator() {
        assert_eq!(find_da1_reply(b"\x1b[?62;22"), None);
        assert_eq!(find_da1_reply(b"xx\x1b[?62;22c"), Some(2));
    }
}
