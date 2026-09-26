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

const HEADED: &str = "time,temp\n0,20\n1,21\n2,23\n";
const BARE: &str = "0,20\n1,21\n2,23\n";

/// Plots `csv` with `args` into a 320x240 PNG and returns its pixels.
fn render(csv: &str, args: &[&str]) -> image::RgbImage {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data.csv");
    std::fs::write(&data, csv).unwrap();
    let png = dir.path().join("plot.png");
    let mut all = vec![
        data.to_str().unwrap(),
        "-o",
        png.to_str().unwrap(),
        "--width",
        "320",
        "--height",
        "240",
    ];
    all.extend_from_slice(args);
    let out = termplt(&all, "");
    assert!(out.status.success(), "{}", stderr(&out));
    image::open(&png).unwrap().to_rgb8()
}

#[test]
fn a_title_is_drawn() {
    assert_ne!(
        render(BARE, &[]),
        render(BARE, &["--title", "Temperatures"])
    );
}

#[test]
fn a_title_may_start_with_a_hyphen() {
    assert_ne!(render(BARE, &["--title", "-5 dB"]), render(BARE, &[]));
}

#[test]
fn axes_are_named_from_the_header_unless_told_otherwise() {
    let bare = render(BARE, &[]);
    // the header names the axes...
    assert_ne!(render(HEADED, &[]), bare);
    // ...an empty --xlabel/--ylabel removes those names...
    assert_eq!(render(HEADED, &["--xlabel", "", "--ylabel", ""]), bare);
    // ...and explicit names replace them
    let named = ["--xlabel", "t", "--ylabel", "T"];
    assert_eq!(render(HEADED, &named), render(BARE, &named));
}

#[test]
fn font_errors_name_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("plot.png");
    let run = |font: &Path| {
        termplt(
            &[
                "-d",
                "(1,1),(2,2)",
                "-o",
                png.to_str().unwrap(),
                "--font",
                font.to_str().unwrap(),
            ],
            "",
        )
    };
    let out = run(&dir.path().join("missing.ttf"));
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("cannot read font"),
        "{}",
        stderr(&out)
    );
    assert!(stderr(&out).contains("missing.ttf"), "{}", stderr(&out));

    let notes = dir.path().join("notes.txt");
    std::fs::write(&notes, "not a font").unwrap();
    let out = run(&notes);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("is not a TrueType or OpenType font"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn font_size_must_be_at_least_one() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("plot.png");
    let args = [
        "-d",
        "(1,1),(2,2)",
        "-o",
        png.to_str().unwrap(),
        "--font-size",
        "0",
    ];
    let out = termplt(&args, "");
    assert!(!out.status.success());
    // clap's range check, not an unknown flag
    assert!(
        stderr(&out).contains("invalid value '0' for '--font-size"),
        "{}",
        stderr(&out)
    );
}
