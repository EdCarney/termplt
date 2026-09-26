//! Series names for the legend.

use std::path::Path;

/// What a series can be named from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameSource {
    /// The `label=` value of its `--series` spec; `Some("")` means no name.
    pub label: Option<String>,
    /// Its file as written on the command line (`-` for stdin); `None` for inline data.
    pub path: Option<String>,
    /// The y column's header name.
    pub header: Option<String>,
    /// The y column's 1-based index.
    pub column: usize,
}

/// Names each series for the legend:
/// 1. a `label=` is used as given (`""` for no name);
/// 2. inline data has no name;
/// 3. a file series is named by its y column's header, else by the file's stem (`stdin` for
///    `-`), as `stem: index` when the file gives several series;
/// 4. a name from rule 3 that equals another name becomes the stem, or `stem: column` when the
///    file gives several series; where two files share that stem, the path as written replaces
///    it.
///
/// Names that still repeat are kept: colors and markers tell those series apart.
pub fn series_names(sources: &[NameSource]) -> Vec<Option<String>> {
    let series_in = |path: &str| {
        (sources.iter())
            .filter(|s| s.path.as_deref() == Some(path))
            .count()
    };
    // the file's name for a series: `file`, plus the column when the file gives several
    let file_name = |source: &NameSource, file: &str| {
        let path = source.path.as_deref().unwrap_or_default();
        if series_in(path) > 1 {
            let column = (source.header.clone()).unwrap_or_else(|| source.column.to_string());
            format!("{file}: {column}")
        } else {
            file.to_string()
        }
    };

    let mut names: Vec<Option<String>> = (sources.iter())
        .map(|source| match (&source.label, &source.path) {
            (Some(label), _) => Some(label.clone()).filter(|l| !l.is_empty()),
            (None, None) => None,
            (None, Some(path)) => {
                Some((source.header.clone()).unwrap_or_else(|| file_name(source, &stem(path))))
            }
        })
        .collect();

    // only automatic names change, and only while they equal another name
    let automatic = |i: usize| sources[i].label.is_none() && sources[i].path.is_some();
    let repeated = |names: &[Option<String>], i: usize| {
        names[i].is_some()
            && (names.iter().enumerate()).any(|(j, other)| j != i && *other == names[i])
    };
    let clashing: Vec<usize> = (0..names.len())
        .filter(|&i| automatic(i) && repeated(&names, i))
        .collect();
    for i in clashing {
        let path = sources[i].path.as_deref().unwrap_or_default();
        names[i] = Some(file_name(&sources[i], &stem(path)));
    }
    let clashing: Vec<usize> = (0..names.len())
        .filter(|&i| automatic(i) && repeated(&names, i))
        .collect();
    for i in clashing {
        let path = sources[i].path.as_deref().unwrap_or_default();
        let shares_stem = (sources.iter().filter_map(|s| s.path.as_deref()))
            .any(|other| other != path && stem(other) == stem(path));
        if shares_stem {
            names[i] = Some(file_name(&sources[i], path));
        }
    }
    names
}

/// A file's name without its directory and extension: `stdin` for `-`, and the path as written
/// when it has no stem.
fn stem(path: &str) -> String {
    if path == "-" {
        return "stdin".to_string();
    }
    (Path::new(path).file_stem())
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, header: Option<&str>, column: usize) -> NameSource {
        NameSource {
            path: Some(path.into()),
            header: header.map(Into::into),
            column,
            ..NameSource::default()
        }
    }

    fn inline() -> NameSource {
        NameSource::default()
    }

    fn labeled(source: NameSource, label: &str) -> NameSource {
        NameSource {
            label: Some(label.into()),
            ..source
        }
    }

    fn check(sources: &[NameSource], expected: &[Option<&str>]) {
        let names = series_names(sources);
        assert_eq!(
            names.iter().map(Option::as_deref).collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn a_label_wins_and_an_empty_one_means_no_name() {
        check(
            &[
                labeled(file("a.csv", Some("value"), 2), "sin"),
                labeled(inline(), ""),
                labeled(file("b.csv", None, 2), ""),
            ],
            &[Some("sin"), None, None],
        );
    }

    #[test]
    fn inline_data_has_no_name() {
        check(
            &[inline(), file("d.csv", Some("value"), 2)],
            &[None, Some("value")],
        );
    }

    #[test]
    fn a_file_series_is_named_by_its_header() {
        check(
            &[file("d.csv", Some("temp"), 2), file("d.csv", Some("rh"), 3)],
            &[Some("temp"), Some("rh")],
        );
    }

    #[test]
    fn without_a_header_the_file_names_its_series() {
        check(&[file("data/d.txt", None, 2)], &[Some("d")]);
        check(
            &[file("d.txt", None, 2), file("d.txt", None, 3)],
            &[Some("d: 2"), Some("d: 3")],
        );
        check(&[file("-", None, 2)], &[Some("stdin")]);
        check(&[file("..", None, 2)], &[Some("..")]);
    }

    #[test]
    fn clashing_headers_fall_back_to_file_names() {
        check(
            &[
                file("a.csv", Some("value"), 2),
                file("b.csv", Some("value"), 2),
            ],
            &[Some("a"), Some("b")],
        );
        check(
            &[
                file("run1.csv", Some("temp"), 2),
                file("run1.csv", Some("rh"), 3),
                file("run2.csv", Some("temp"), 2),
                file("run2.csv", Some("rh"), 3),
            ],
            &[
                Some("run1: temp"),
                Some("run1: rh"),
                Some("run2: temp"),
                Some("run2: rh"),
            ],
        );
    }

    #[test]
    fn only_clashing_names_change() {
        check(
            &[
                file("a.csv", Some("temp"), 2),
                file("b.csv", Some("temp"), 2),
                file("c.csv", Some("rh"), 2),
            ],
            &[Some("a"), Some("b"), Some("rh")],
        );
    }

    #[test]
    fn a_label_is_never_renamed() {
        check(
            &[file("a.csv", Some("value"), 2), labeled(inline(), "value")],
            &[Some("a"), Some("value")],
        );
    }

    #[test]
    fn files_that_share_a_stem_use_their_paths() {
        check(
            &[
                file("runs/1/d.csv", Some("value"), 2),
                file("runs/2/d.csv", Some("value"), 2),
            ],
            &[Some("runs/1/d.csv"), Some("runs/2/d.csv")],
        );
    }

    #[test]
    fn names_that_still_repeat_are_kept() {
        // the same column twice
        check(
            &[file("d.csv", Some("v"), 2), file("d.csv", Some("v"), 2)],
            &[Some("d: v"), Some("d: v")],
        );
        // the same headerless file twice
        check(
            &[file("a.csv", None, 2), file("a.csv", None, 2)],
            &[Some("a: 2"), Some("a: 2")],
        );
    }
}
