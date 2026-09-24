//! Parsing of inline points and delimited data files.

use crate::Result;
use termplt::plotting::point::Point;

/// A column of a data file.
#[derive(Debug, Clone, PartialEq)]
pub enum Column {
    /// Zero-based column index.
    Index(usize),
    /// Header name (matched ignoring case and surrounding quotes).
    Name(String),
    /// The zero-based row number of each data row.
    RowNumber,
}

impl Column {
    /// Parses a column reference: a 1-based index, "index" for the row number, or a name.
    pub fn parse(value: &str) -> Result<Column> {
        let value = value.trim();
        if value.eq_ignore_ascii_case("index") {
            return Ok(Column::RowNumber);
        }
        match value.parse::<usize>() {
            Ok(0) => Err("column indices start at 1".into()),
            Ok(n) => Ok(Column::Index(n - 1)),
            Err(_) if value.is_empty() => Err("column name is empty".into()),
            Err(_) => Ok(Column::Name(value.to_string())),
        }
    }
}

/// Points read from a source, plus the number of rows skipped for missing values.
#[derive(Debug, Default)]
pub struct Parsed {
    pub points: Vec<Point<f64>>,
    pub skipped_missing: usize,
}

/// Parses inline points: "(1,2),(3,4)", "(1, 2) (3, 4)", "1,2 3,4" or "1,2;3,4".
pub fn parse_inline(s: &str) -> Result<Vec<Point<f64>>> {
    let s = s.trim();
    if s.is_empty() {
        return Err("inline data is empty".into());
    }

    // without parentheses, points are separated by whitespace or ';', so first join "1, 2"
    // into "1,2"
    let joined;
    let groups: Vec<&str> = if s.contains('(') {
        parenthesized_groups(s)?
    } else {
        joined = s
            .split(',')
            .map(str::trim)
            .collect::<Vec<_>>()
            .join(",");
        joined
            .split(|c: char| c.is_whitespace() || c == ';')
            .filter(|g| !g.is_empty())
            .collect()
    };

    groups
        .into_iter()
        .map(|group| {
            let values: Vec<&str> = group
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|v| !v.is_empty())
                .collect();
            let [x, y] = values[..] else {
                return Err(format!(
                    "invalid point '{group}': expected two numbers, e.g. (1,2) or 1,2"
                )
                .into());
            };
            let parse = |v: &str, axis: &str| -> Result<f64> {
                v.parse::<f64>()
                    .map_err(|_| format!("cannot parse {axis} value '{v}' as a number").into())
            };
            Ok(Point::new(parse(x, "x")?, parse(y, "y")?))
        })
        .collect()
}

/// Returns the contents of each "( ... )" group, requiring only separators between groups.
fn parenthesized_groups(s: &str) -> Result<Vec<&str>> {
    let mut groups = Vec::new();
    let mut rest = s;
    loop {
        let rest_trimmed =
            rest.trim_start_matches(|c: char| c.is_whitespace() || c == ',' || c == ';');
        if rest_trimmed.is_empty() {
            return Ok(groups);
        }
        let Some(inner) = rest_trimmed.strip_prefix('(') else {
            return Err(format!(
                "unexpected '{}' in inline data: every point must be written as (x,y)",
                rest_trimmed.chars().next().unwrap_or(' ')
            )
            .into());
        };
        let Some(end) = inner.find(')') else {
            return Err("missing ')' in inline data".into());
        };
        groups.push(&inner[..end]);
        rest = &inner[end + 1..];
    }
}

/// Values treated as missing (compared ignoring case). Rows with a missing x or y are skipped.
const MISSING: &[&str] = &["", "na", "n/a", "null", "none", "-", "?"];

fn is_missing(token: &str) -> bool {
    MISSING.iter().any(|m| token.eq_ignore_ascii_case(m))
}

/// A delimited table: an optional header and the data rows with their 1-based line numbers.
#[derive(Debug)]
pub struct Table {
    pub header: Option<Vec<String>>,
    pub rows: Vec<(usize, Vec<String>)>,
}

impl Table {
    /// Parses CSV, TSV or whitespace-delimited text. Blank lines and lines starting with '#'
    /// are ignored. The first row is a header if any of its fields is neither a number nor a
    /// missing value.
    pub fn parse(content: &str) -> Table {
        let mut rows: Vec<(usize, Vec<String>)> = content
            .lines()
            .enumerate()
            .filter(|(_, line)| {
                let line = line.trim();
                !line.is_empty() && !line.starts_with('#')
            })
            .map(|(i, line)| (i + 1, split_fields(line)))
            .collect();

        let has_header = rows.first().is_some_and(|(_, fields)| {
            fields
                .iter()
                .any(|f| !is_missing(f) && f.parse::<f64>().is_err())
        });
        let header = if has_header {
            Some(rows.remove(0).1)
        } else {
            None
        };
        Table { header, rows }
    }

    fn width(&self) -> usize {
        self.rows.iter().map(|(_, r)| r.len()).max().unwrap_or(0)
    }

    fn resolve(&self, column: &Column, source: &str) -> Result<Column> {
        match column {
            Column::Name(name) => {
                let Some(header) = &self.header else {
                    return Err(format!(
                        "{source} has no header row, so column '{name}' cannot be found; use a \
                         1-based column number instead"
                    )
                    .into());
                };
                header
                    .iter()
                    .position(|h| h.eq_ignore_ascii_case(name))
                    .map(Column::Index)
                    .ok_or_else(|| {
                        format!(
                            "{source} has no column '{name}'; available columns: {}",
                            header.join(", ")
                        )
                        .into()
                    })
            }
            other => Ok(other.clone()),
        }
    }

    fn column_label(&self, column: &Column) -> String {
        match column {
            Column::Index(i) => match self.header.as_ref().and_then(|h| h.get(*i)) {
                Some(name) => format!("'{name}'"),
                None => format!("{}", i + 1),
            },
            Column::Name(name) => format!("'{name}'"),
            Column::RowNumber => "index".to_string(),
        }
    }

    /// Extracts (x, y) points. With no columns given, a single-column table is plotted
    /// against the row number and wider tables use columns 1 and 2.
    pub fn points(&self, x: Option<&Column>, y: Option<&Column>, source: &str) -> Result<Parsed> {
        let single_column = self.width() == 1;
        let x = match x {
            Some(x) => self.resolve(x, source)?,
            None if single_column && y.is_none() => Column::RowNumber,
            None => Column::Index(0),
        };
        let y = match y {
            Some(y) => self.resolve(y, source)?,
            None if single_column => Column::Index(0),
            None => Column::Index(1),
        };
        if y == Column::RowNumber {
            return Err("the y column cannot be 'index'".into());
        }

        let mut parsed = Parsed::default();
        for (row_number, (line, fields)) in self.rows.iter().enumerate() {
            let value = |column: &Column, axis: &str| -> Result<Option<f64>> {
                let token = match column {
                    Column::RowNumber => return Ok(Some(row_number as f64)),
                    Column::Index(i) => fields.get(*i).ok_or_else(|| {
                        format!(
                            "{source}:{line}: {axis} column {} is missing (the row has {} \
                             field(s))",
                            self.column_label(column),
                            fields.len()
                        )
                    })?,
                    Column::Name(_) => unreachable!("names are resolved to indices"),
                };
                if is_missing(token) {
                    return Ok(None);
                }
                token.parse::<f64>().map(Some).map_err(|_| {
                    format!(
                        "{source}:{line}: cannot parse {axis} value '{token}' (column {}) as a \
                         number",
                        self.column_label(column)
                    )
                    .into()
                })
            };
            match (value(&x, "x")?, value(&y, "y")?) {
                (Some(x), Some(y)) => parsed.points.push(Point::new(x, y)),
                _ => parsed.skipped_missing += 1,
            }
        }
        Ok(parsed)
    }
}

/// Splits a line on commas, else tabs, else runs of whitespace. Empty fields are kept for
/// comma/tab-separated lines so columns do not shift; surrounding quotes are removed.
fn split_fields(line: &str) -> Vec<String> {
    let line = line.trim();
    let fields: Vec<&str> = if line.contains(',') {
        line.split(',').collect()
    } else if line.contains('\t') {
        line.split('\t').collect()
    } else {
        line.split_whitespace().collect()
    };
    fields
        .into_iter()
        .map(|f| {
            let f = f.trim();
            f.strip_prefix('"')
                .and_then(|f| f.strip_suffix('"'))
                .unwrap_or(f)
                .to_string()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(v: &[(f64, f64)]) -> Vec<Point<f64>> {
        v.iter().map(|&(x, y)| Point::new(x, y)).collect()
    }

    // -- inline data --

    #[test]
    fn inline_formats() {
        let expected = pts(&[(1.0, 2.0), (3.0, -4.5)]);
        for s in [
            "(1,2),(3,-4.5)",
            "(1, 2), (3, -4.5)",
            " ( 1 , 2 )( 3 , -4.5 ) ",
            "(1 2) (3 -4.5)",
            "1,2 3,-4.5",
            "1,2;3,-4.5",
            "1, 2; 3, -4.5",
        ] {
            assert_eq!(parse_inline(s).unwrap(), expected, "{s}");
        }
    }

    #[test]
    fn inline_single_point() {
        assert_eq!(parse_inline("(1.5,2.5)").unwrap(), pts(&[(1.5, 2.5)]));
    }

    #[test]
    fn inline_errors() {
        for s in ["", "(1,2,3)", "(1,2", "(1,2) x (3,4)", "(a,2)", "1,2,3,4"] {
            assert!(parse_inline(s).is_err(), "{s}");
        }
    }

    // -- columns --

    #[test]
    fn column_parsing() {
        assert_eq!(Column::parse("1").unwrap(), Column::Index(0));
        assert_eq!(
            Column::parse(" Temp ").unwrap(),
            Column::Name("Temp".into())
        );
        assert_eq!(Column::parse("INDEX").unwrap(), Column::RowNumber);
        assert!(Column::parse("0").is_err());
    }

    // -- tables --

    fn table_points(content: &str, x: Option<&str>, y: Option<&str>) -> Result<Parsed> {
        let x = x.map(|c| Column::parse(c).unwrap());
        let y = y.map(|c| Column::parse(c).unwrap());
        Table::parse(content).points(x.as_ref(), y.as_ref(), "data.csv")
    }

    #[test]
    fn csv_without_header() {
        let parsed = table_points("1,2\n3,4\n", None, None).unwrap();
        assert_eq!(parsed.points, pts(&[(1.0, 2.0), (3.0, 4.0)]));
    }

    #[test]
    fn csv_with_header_comments_and_blanks() {
        let parsed = table_points("# comment\nx,y\n\n1,2\n# more\n3,4\n", None, None).unwrap();
        assert_eq!(parsed.points, pts(&[(1.0, 2.0), (3.0, 4.0)]));
    }

    #[test]
    fn tab_and_whitespace_delimited() {
        let tsv = table_points("1\t2\n3\t4\n", None, None).unwrap();
        let ws = table_points("1   2\n  3 4  \n", None, None).unwrap();
        assert_eq!(tsv.points, pts(&[(1.0, 2.0), (3.0, 4.0)]));
        assert_eq!(ws.points, tsv.points);
    }

    #[test]
    fn columns_by_name_and_index() {
        let csv = "time,temp,\"humidity\"\n0,20,50\n1,21,55\n";
        let by_name = table_points(csv, Some("time"), Some("HUMIDITY")).unwrap();
        let by_index = table_points(csv, Some("1"), Some("3")).unwrap();
        assert_eq!(by_name.points, pts(&[(0.0, 50.0), (1.0, 55.0)]));
        assert_eq!(by_index.points, by_name.points);
    }

    #[test]
    fn unknown_column_lists_available_columns() {
        let err = table_points("a,b\n1,2\n", None, Some("c"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("available columns: a, b"), "{err}");
    }

    #[test]
    fn column_name_without_header_errors() {
        let err = table_points("1,2\n", None, Some("temp"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("no header"), "{err}");
    }

    #[test]
    fn single_column_is_plotted_against_row_number() {
        let parsed = table_points("value\n5\n7\n9\n", None, None).unwrap();
        assert_eq!(parsed.points, pts(&[(0.0, 5.0), (1.0, 7.0), (2.0, 9.0)]));
    }

    #[test]
    fn explicit_row_number_x() {
        let parsed = table_points("1,10\n2,20\n", Some("index"), Some("2")).unwrap();
        assert_eq!(parsed.points, pts(&[(0.0, 10.0), (1.0, 20.0)]));
    }

    #[test]
    fn missing_values_are_skipped_and_counted() {
        let parsed = table_points("x,y\n1,2\n2,\n3,NA\n4,n/a\n,5\n6,7\n", None, None).unwrap();
        assert_eq!(parsed.points, pts(&[(1.0, 2.0), (6.0, 7.0)]));
        assert_eq!(parsed.skipped_missing, 4);
    }

    #[test]
    fn empty_fields_do_not_shift_columns() {
        let parsed = table_points("a,b,c\n1,,3\n4,5,6\n", Some("1"), Some("3")).unwrap();
        assert_eq!(parsed.points, pts(&[(1.0, 3.0), (4.0, 6.0)]));
    }

    #[test]
    fn parse_errors_report_file_line_and_column() {
        let err = table_points("x,y\n1,2\nfoo,3\n", None, None)
            .unwrap_err()
            .to_string();
        assert!(err.starts_with("data.csv:3:"), "{err}");
        assert!(err.contains("'foo'") && err.contains("'x'"), "{err}");
    }

    #[test]
    fn short_rows_report_the_missing_column() {
        let err = table_points("1,2\n3\n", None, None)
            .unwrap_err()
            .to_string();
        assert!(err.starts_with("data.csv:2:"), "{err}");
    }

    #[test]
    fn y_cannot_be_the_row_number() {
        assert!(table_points("1,2\n", None, Some("index")).is_err());
    }
}
