//! End-to-end tests of the CLI that need no terminal: PNG output, files, stdin and errors.

use std::{
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

fn termplt(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_termplt"))
        .args(args)
        .env_remove("TMUX")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn png_size(path: &Path) -> (u32, u32) {
    let img = image::open(path).unwrap();
    (img.width(), img.height())
}

#[test]
fn writes_a_png_from_a_csv_file() {
    let dir = tempfile::tempdir().unwrap();
    let csv = dir.path().join("sensors.csv");
    std::fs::write(&csv, "time,temp,humidity\n0,20,50\n1,21,\n2,23,52\n").unwrap();
    let png = dir.path().join("plot.png");
    let out = termplt(
        &[
            csv.to_str().unwrap(),
            "-x",
            "time",
            "-y",
            "temp,humidity",
            "-o",
            png.to_str().unwrap(),
            "--width",
            "320",
            "--height",
            "240",
        ],
        "",
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(png_size(&png), (320, 240));
    // nothing meant for a terminal is written
    assert!(out.stdout.is_empty());
}

#[test]
fn reads_piped_stdin() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("plot.png");
    let out = termplt(&["-o", png.to_str().unwrap()], "1 1\n2 4\n3 9\n");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(png_size(&png), (800, 600));
}

#[test]
fn not_a_terminal_suggests_output() {
    let out = termplt(&["--data", "(1,1),(2,4)"], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("not a terminal"), "{err}");
    assert!(err.contains("--output"), "{err}");
    // no escape sequences leak into the pipe
    assert!(!out.stdout.contains(&0x1b));
}

#[test]
fn rejects_non_png_output_before_reading_data() {
    let out = termplt(&["missing.csv", "-o", "plot.jpg"], "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("only PNG output"), "{}", stderr(&out));
}

#[test]
fn bad_data_is_reported_with_its_line() {
    let dir = tempfile::tempdir().unwrap();
    let csv = dir.path().join("bad.csv");
    std::fs::write(&csv, "x,y\n1,2\n3,oops\n").unwrap();
    let png = dir.path().join("plot.png");
    let out = termplt(&[csv.to_str().unwrap(), "-o", png.to_str().unwrap()], "");
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("bad.csv:3:"), "{err}");
    assert!(!png.exists());
}
