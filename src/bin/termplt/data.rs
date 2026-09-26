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
    pub names: ColumnNames,
    /// The y column's 1-based index.
    pub y_column: usize,
}

/// The header names of the columns a series was read from; `None` where a column has none.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ColumnNames {
    pub x: Option<String>,
    pub y: Option<String>,
}

/// The names to put on the axes: an axis is named only when every series has a name for it and
/// all the names are the same (compared exactly).
pub fn axis_names(series: &[ColumnNames]) -> ColumnNames {
    fn agreed<'a>(mut names: impl Iterator<Item = &'a Option<String>>) -> Option<String> {
        let first = names.next()?.clone()?;
        names
            .all(|name| name.as_deref() == Some(first.as_str()))
            .then_some(first)
    }
    ColumnNames {
        x: agreed(series.iter().map(|s| &s.x)),
        y: agreed(series.iter().map(|s| &s.y)),
    }
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
        joined = s.split(',').map(str::trim).collect::<Vec<_>>().join(",");
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

/// A data row: its 1-based line number and its fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub line: usize,
    pub fields: Vec<String>,
}

/// Reads a delimited table one line at a time, with the rules of `Table::parse`.
#[derive(Debug, Default)]
pub struct TableReader {
    header: Option<Vec<String>>,
    /// The field count of the header or the first data row; `None` until one has been read.
    width: Option<usize>,
    /// The number of lines read so far.
    lines: usize,
}

impl TableReader {
    pub fn new() -> TableReader {
        TableReader::default()
    }

    /// Feeds one line (without its terminator). Returns the data row it holds, or `None` for a
    /// blank line, a `#` comment, or the header.
    pub fn push_line(&mut self, line: &str) -> Option<Row> {
        self.lines += 1;
        // spreadsheet exports often start with a BOM, which would make the first row look
        // like a header
        let line = match self.lines {
            1 => line.strip_prefix('\u{feff}').unwrap_or(line),
            _ => line,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        let fields = split_fields(trimmed);
        if self.width.is_none() {
            self.width = Some(fields.len());
            // the first row is a header if any of its fields is neither a number nor a
            // missing value
            if fields
                .iter()
                .any(|f| !is_missing(f) && f.parse::<f64>().is_err())
            {
                self.header = Some(fields);
                return None;
            }
        }
        Some(Row {
            line: self.lines,
            fields,
        })
    }

    /// The header, once the first non-blank, non-comment line has been seen and was one.
    pub fn header(&self) -> Option<&[String]> {
        self.header.as_deref()
    }

    /// The field count of the header, or of the first data row (for the single-column rule).
    pub fn width(&self) -> Option<usize> {
        self.width
    }
}

/// A delimited table: an optional header and the data rows with their 1-based line numbers.
#[derive(Debug)]
pub struct Table {
    pub header: Option<Vec<String>>,
    pub rows: Vec<Row>,
}

impl Table {
    /// Parses comma-, semicolon-, tab- or whitespace-delimited text. A UTF-8 byte order mark
    /// is skipped, and blank lines and lines starting with '#' are ignored. The first row is a
    /// header if any of its fields is neither a number nor a missing value.
    pub fn parse(content: &str) -> Table {
        let mut reader = TableReader::new();
        let rows = content
            .lines()
            .filter_map(|line| reader.push_line(line))
            .collect();
        Table {
            header: reader.header().map(<[String]>::to_vec),
            rows,
        }
    }

    /// The field count of the widest data row.
    ///
    /// This is the one rule a stream cannot follow: `TableReader::width` knows only the header
    /// or the first data row. The two differ only for ragged tables (a two-field header over
    /// one-field rows, or a one-field first row followed by wider ones), where they can disagree
    /// about the single-column rule. A whole table keeps the widest row so that its output does
    /// not change.
    fn width(&self) -> usize {
        self.rows.iter().map(|r| r.fields.len()).max().unwrap_or(0)
    }

    /// Extracts (x, y) points. Without an x column, a single-column table is plotted against
    /// the row number and wider tables use column 1 for x (and column 2 for y by default).
    pub fn points(&self, x: Option<&Column>, y: Option<&Column>, source: &str) -> Result<Parsed> {
        let columns = Columns::resolve(self.header.as_deref(), self.width(), x, y, source)?;
        let mut parsed = Parsed {
            names: columns.names.clone(),
            y_column: columns.y_column,
            ..Parsed::default()
        };
        for (row_number, row) in self.rows.iter().enumerate() {
            match columns.point(row_number, row, source)? {
                Some(point) => parsed.points.push(point),
                None => parsed.skipped_missing += 1,
            }
        }
        Ok(parsed)
    }
}

/// A resolved table column: its field index and how error messages name it.
#[derive(Debug, Clone, PartialEq)]
struct Field {
    /// Zero-based field index.
    index: usize,
    /// The quoted header name, or the 1-based index when the header has none.
    label: String,
}

impl Field {
    fn new(header: Option<&[String]>, index: usize) -> Field {
        let label = match header.and_then(|h| h.get(index)) {
            Some(name) => format!("'{name}'"),
            None => format!("{}", index + 1),
        };
        Field { index, label }
    }

    /// The field's value in `row`: `None` for a missing value, an error for a missing field or
    /// a value that is not a number.
    fn value(&self, row: &Row, axis: &str, source: &str) -> Result<Option<f64>> {
        let line = row.line;
        let Some(token) = row.fields.get(self.index) else {
            return Err(format!(
                "{source}:{line}: {axis} column {} is missing (the row has {} field(s))",
                self.label,
                row.fields.len()
            )
            .into());
        };
        if is_missing(token) {
            return Ok(None);
        }
        token.parse::<f64>().map(Some).map_err(|_| {
            format!(
                "{source}:{line}: cannot parse {axis} value '{token}' (column {}) as a number",
                self.label
            )
            .into()
        })
    }
}

/// Resolved x and y columns of a source, with the names they take from the header.
#[derive(Debug, Clone, PartialEq)]
pub struct Columns {
    /// The x column, or `None` for the row number.
    x: Option<Field>,
    y: Field,
    pub names: ColumnNames,
    /// The y column's 1-based index.
    pub y_column: usize,
}

impl Columns {
    /// Resolves the requested columns against a header (if any) and the table width. Without an
    /// x column, a single-column table is plotted against the row number and wider tables use
    /// column 1 for x (and column 2 for y by default).
    pub fn resolve(
        header: Option<&[String]>,
        width: usize,
        x: Option<&Column>,
        y: Option<&Column>,
        source: &str,
    ) -> Result<Columns> {
        let single_column = width == 1;
        let x = match x {
            Some(x) => field_index(header, x, source)?,
            // the only column is y, whether chosen or not
            None if single_column => None,
            None => Some(0),
        };
        let y = match y {
            Some(y) => field_index(header, y, source)?,
            None if single_column => Some(0),
            None => Some(1),
        };
        let Some(y) = y else {
            return Err("the y column cannot be 'index'".into());
        };
        Ok(Columns {
            x: x.map(|x| Field::new(header, x)),
            y: Field::new(header, y),
            names: ColumnNames {
                x: x.and_then(|x| header_name(header, x)),
                y: header_name(header, y),
            },
            y_column: y + 1,
        })
    }

    /// One row's point: `Ok(None)` when x or y is a missing value (the caller counts it), and
    /// an error naming the source and line for a missing field or a value that is not a number.
    /// `row_number` is the zero-based index of the data row, the x value of `Column::RowNumber`.
    pub fn point(&self, row_number: usize, row: &Row, source: &str) -> Result<Option<Point<f64>>> {
        let x = match &self.x {
            Some(x) => x.value(row, "x", source)?,
            None => Some(row_number as f64),
        };
        let y = self.y.value(row, "y", source)?;
        Ok(x.zip(y).map(|(x, y)| Point::new(x, y)))
    }
}

/// The zero-based field index of a column, or `None` for the row number.
fn field_index(header: Option<&[String]>, column: &Column, source: &str) -> Result<Option<usize>> {
    match column {
        Column::Index(i) => Ok(Some(*i)),
        Column::RowNumber => Ok(None),
        Column::Name(name) => {
            let Some(header) = header else {
                return Err(format!(
                    "{source} has no header row, so column '{name}' cannot be found; use a \
                     1-based column number instead"
                )
                .into());
            };
            header
                .iter()
                .position(|h| h.eq_ignore_ascii_case(name))
                .map(Some)
                .ok_or_else(|| {
                    format!(
                        "{source} has no column '{name}'; available columns: {}",
                        header.join(", ")
                    )
                    .into()
                })
        }
    }
}

/// The header text of a column; `None` without a header or for an empty header cell.
fn header_name(header: Option<&[String]>, index: usize) -> Option<String> {
    let name = header?.get(index)?.trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// Splits a line on commas, else tabs, else runs of whitespace. Empty fields are kept for
/// comma/tab-separated lines so columns do not shift; surrounding quotes are removed.
fn split_fields(line: &str) -> Vec<String> {
    let line = line.trim();
    let fields: Vec<&str> = if line.contains(',') {
        line.split(',').collect()
    } else if line.contains(';') {
        line.split(';').collect()
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

    #[test]
    fn points_report_the_y_column() {
        let table = Table::parse("a,b,c\n1,2,3\n");
        assert_eq!(table.points(None, None, "t").unwrap().y_column, 2);
        let c = Column::Name("c".into());
        assert_eq!(table.points(None, Some(&c), "t").unwrap().y_column, 3);
        assert_eq!(
            Table::parse("5\n6\n")
                .points(None, None, "t")
                .unwrap()
                .y_column,
            1
        );
    }

    fn pts(v: &[(f64, f64)]) -> Vec<Point<f64>> {
        v.iter().map(|&(x, y)| Point::new(x, y)).collect()
    }

    fn names(x: Option<&str>, y: Option<&str>) -> ColumnNames {
        ColumnNames {
            x: x.map(str::to_string),
            y: y.map(str::to_string),
        }
    }

    #[test]
    fn points_report_the_header_names_of_their_columns() {
        let table = Table::parse("time,temp,humidity\n0,20,50\n1,21,51\n");
        let named = |x: Option<Column>, y: Option<Column>| {
            table.points(x.as_ref(), y.as_ref(), "t").unwrap().names
        };
        assert_eq!(named(None, None), names(Some("time"), Some("temp")));
        // -x 3
        assert_eq!(
            named(Some(Column::Index(2)), None),
            names(Some("humidity"), Some("temp"))
        );
        // names match ignoring case, but the header's own text is reported
        assert_eq!(
            named(None, Some(Column::Name("HUMIDITY".into()))),
            names(Some("time"), Some("humidity"))
        );
        // -x index
        assert_eq!(
            named(Some(Column::RowNumber), None),
            names(None, Some("temp"))
        );
    }

    #[test]
    fn columns_without_a_header_name_have_none() {
        let named = |content: &str| Table::parse(content).points(None, None, "t").unwrap().names;
        assert_eq!(named("0,20\n1,21\n"), ColumnNames::default());
        // a single column is plotted against the row number
        assert_eq!(named("temp\n20\n21\n"), names(None, Some("temp")));
        assert_eq!(named("time,\n0,20\n"), names(Some("time"), None));
        // quoted and padded header cells give clean names
        assert_eq!(
            named("\"time (s)\" , temp\n0,20\n"),
            names(Some("time (s)"), Some("temp"))
        );
    }

    #[test]
    fn axes_are_named_only_when_every_series_agrees() {
        let temps = names(Some("time"), Some("temp"));
        let cases = [
            (vec![temps.clone()], names(Some("time"), Some("temp"))),
            // -y temp,humidity: two different y names
            (
                vec![temps.clone(), names(Some("time"), Some("humidity"))],
                names(Some("time"), None),
            ),
            // two files with the same header
            (
                vec![temps.clone(), temps.clone()],
                names(Some("time"), Some("temp")),
            ),
            (
                vec![temps.clone(), names(Some("t"), Some("temp"))],
                names(None, Some("temp")),
            ),
            // inline data has no names
            (
                vec![temps.clone(), ColumnNames::default()],
                names(None, None),
            ),
            // compared exactly
            (
                vec![names(Some("Time"), Some("temp")), temps.clone()],
                names(None, Some("temp")),
            ),
            (vec![], names(None, None)),
        ];
        for (series, expected) in cases {
            assert_eq!(axis_names(&series), expected, "{series:?}");
        }
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

    #[test]
    fn byte_order_mark_is_skipped() {
        let table = Table::parse("\u{feff}1,2\n3,4\n5,6\n");
        assert!(table.header.is_none());
        assert_eq!(table.rows.len(), 3);
        let table = Table::parse("\u{feff}time,temp\n0,20\n");
        assert_eq!(
            table.header.as_deref(),
            Some(&["time".to_string(), "temp".into()][..])
        );
    }

    #[test]
    fn semicolon_separated() {
        let parsed = Table::parse("x;y\n1;2\n3;4\n")
            .points(None, None, "t.csv")
            .unwrap();
        assert_eq!(parsed.points, pts(&[(1.0, 2.0), (3.0, 4.0)]));
    }

    #[test]
    fn single_column_with_explicit_y_uses_the_row_number() {
        let parsed = Table::parse("5\n7\n9\n")
            .points(None, Some(&Column::Index(0)), "t.csv")
            .unwrap();
        assert_eq!(parsed.points, pts(&[(0.0, 5.0), (1.0, 7.0), (2.0, 9.0)]));
    }

    #[test]
    fn a_whole_table_takes_its_width_from_the_widest_data_row() {
        // the header has two fields but every data row one, so the single-column rule applies
        // (a stream, knowing only the header, would read two columns here)
        let parsed = table_points("a,b\n5\n7\n", None, None).unwrap();
        assert_eq!(parsed.points, pts(&[(0.0, 5.0), (1.0, 7.0)]));
    }

    // -- line reader --

    fn strings(fields: &[&str]) -> Vec<String> {
        fields.iter().map(|f| f.to_string()).collect()
    }

    fn row(line: usize, fields: &[&str]) -> Row {
        Row {
            line,
            fields: strings(fields),
        }
    }

    /// Feeds `lines` to a new reader, returning it and what each line gave.
    fn read_lines(lines: &[&str]) -> (TableReader, Vec<Option<Row>>) {
        let mut reader = TableReader::new();
        let rows = lines.iter().map(|line| reader.push_line(line)).collect();
        (reader, rows)
    }

    #[test]
    fn reader_skips_a_byte_order_mark_on_the_first_line() {
        let (reader, rows) = read_lines(&["\u{feff}1,2", "3,4"]);
        assert_eq!(reader.header(), None);
        assert_eq!(rows, [Some(row(1, &["1", "2"])), Some(row(2, &["3", "4"]))]);

        let (reader, rows) = read_lines(&["\u{feff}time,temp", "0,20"]);
        assert_eq!(reader.header(), Some(&strings(&["time", "temp"])[..]));
        assert_eq!(rows, [None, Some(row(2, &["0", "20"]))]);

        // a BOM on a blank first line is skipped too, and the header comes on line 2
        let (reader, rows) = read_lines(&["\u{feff}", "time,temp"]);
        assert_eq!(reader.header(), Some(&strings(&["time", "temp"])[..]));
        assert_eq!(rows, [None, None]);
    }

    #[test]
    fn reader_keeps_a_byte_order_mark_after_the_first_line() {
        let (_, rows) = read_lines(&["1,2", "\u{feff}3,4"]);
        assert_eq!(rows[1], Some(row(2, &["\u{feff}3", "4"])));
    }

    #[test]
    fn reader_ignores_blank_and_comment_lines_but_counts_them() {
        let (reader, rows) = read_lines(&[
            "# comment",
            "",
            "   ",
            "x,y",
            "  # indented comment",
            "1,2",
            "\t",
            "3,4",
        ]);
        assert_eq!(reader.header(), Some(&strings(&["x", "y"])[..]));
        let data: Vec<Row> = rows.into_iter().flatten().collect();
        assert_eq!(data, [row(6, &["1", "2"]), row(8, &["3", "4"])]);
    }

    #[test]
    fn reader_takes_a_numeric_first_row_as_data() {
        for (line, fields) in [
            ("1,2", &["1", "2"][..]),
            ("1e3, -2.5, inf, NaN", &["1e3", "-2.5", "inf", "NaN"]),
        ] {
            let (reader, rows) = read_lines(&[line]);
            assert_eq!(reader.header(), None, "{line}");
            assert_eq!(rows, [Some(row(1, fields))], "{line}");
        }
    }

    #[test]
    fn reader_takes_a_first_row_of_missing_values_as_data() {
        let (reader, rows) = read_lines(&["NA,-,?", "1,2,3"]);
        assert_eq!(reader.header(), None);
        assert_eq!(
            rows,
            [
                Some(row(1, &["NA", "-", "?"])),
                Some(row(2, &["1", "2", "3"]))
            ]
        );
        let (reader, _) = read_lines(&[",n/a"]);
        assert_eq!(reader.header(), None);
    }

    #[test]
    fn reader_takes_a_first_row_with_any_text_as_the_header() {
        let (reader, rows) = read_lines(&["1,temp,NA", "0,20,5"]);
        assert_eq!(reader.header(), Some(&strings(&["1", "temp", "NA"])[..]));
        assert_eq!(rows, [None, Some(row(2, &["0", "20", "5"]))]);
    }

    #[test]
    fn reader_looks_for_a_header_only_in_the_first_row() {
        let (reader, rows) = read_lines(&["1,2", "x,y"]);
        assert_eq!(reader.header(), None);
        assert_eq!(rows[1], Some(row(2, &["x", "y"])));
    }

    #[test]
    fn reader_width_is_the_header_or_first_data_row_width() {
        let (reader, _) = read_lines(&[]);
        assert_eq!(reader.width(), None);
        let (reader, _) = read_lines(&["", "# only comments"]);
        assert_eq!(reader.width(), None);
        // later rows, wider or narrower, do not change it
        let (reader, _) = read_lines(&["a,b,c", "1,2", "1,2,3,4"]);
        assert_eq!(reader.width(), Some(3));
        let (reader, _) = read_lines(&["# data", "5", "6,7"]);
        assert_eq!(reader.width(), Some(1));
        let (reader, _) = read_lines(&["1,2", "3"]);
        assert_eq!(reader.width(), Some(2));
    }

    #[test]
    fn reader_splits_fields_on_the_first_delimiter_found() {
        let cases: &[(&str, &[&str])] = &[
            // commas win over everything else
            ("1,2;3\t4 5", &["1", "2;3\t4 5"]),
            // then semicolons
            ("1;2\t3 4", &["1", "2\t3 4"]),
            // then tabs
            ("1\t2 3", &["1", "2 3"]),
            // then runs of whitespace
            ("  1   2 3  ", &["1", "2", "3"]),
            // empty fields keep their place with commas, semicolons and tabs
            ("1,,3", &["1", "", "3"]),
            ("1;;3", &["1", "", "3"]),
            ("1\t\t3", &["1", "", "3"]),
            // fields are trimmed
            (" 1 , 2 ", &["1", "2"]),
        ];
        for &(line, fields) in cases {
            // after a first row, every line is data, numbers or not
            let (_, rows) = read_lines(&["0", line]);
            assert_eq!(rows[1], Some(row(2, fields)), "{line:?}");
        }
    }

    #[test]
    fn reader_strips_surrounding_quotes() {
        let (reader, rows) = read_lines(&["\"time (s)\" , \"temp\",\"\"", "\"0\",\"20\",\"\""]);
        assert_eq!(
            reader.header(),
            Some(&strings(&["time (s)", "temp", ""])[..])
        );
        assert_eq!(rows[1], Some(row(2, &["0", "20", ""])));
    }

    // -- column resolution --

    fn resolve(
        header: Option<&[&str]>,
        width: usize,
        x: Option<&str>,
        y: Option<&str>,
    ) -> Result<Columns> {
        let header = header.map(strings);
        let x = x.map(|c| Column::parse(c).unwrap());
        let y = y.map(|c| Column::parse(c).unwrap());
        Columns::resolve(header.as_deref(), width, x.as_ref(), y.as_ref(), "t")
    }

    fn point_of(columns: &Columns, row_number: usize, fields: &[&str]) -> Option<(f64, f64)> {
        columns
            .point(row_number, &row(1, fields), "t")
            .unwrap()
            .map(|p| (p.x, p.y))
    }

    #[test]
    fn columns_default_to_the_first_two() {
        let columns = resolve(None, 3, None, None).unwrap();
        assert_eq!(point_of(&columns, 0, &["1", "2", "3"]), Some((1.0, 2.0)));
        assert_eq!(columns.y_column, 2);
        assert_eq!(columns.names, ColumnNames::default());

        let columns = resolve(Some(&["a", "b", "c"]), 3, None, None).unwrap();
        assert_eq!(columns.names, names(Some("a"), Some("b")));
    }

    #[test]
    fn a_single_column_is_y_against_the_row_number() {
        let columns = resolve(None, 1, None, None).unwrap();
        assert_eq!(point_of(&columns, 4, &["7"]), Some((4.0, 7.0)));
        assert_eq!(columns.y_column, 1);
        assert_eq!(columns.names, ColumnNames::default());

        let columns = resolve(Some(&["temp"]), 1, None, None).unwrap();
        assert_eq!(columns.names, names(None, Some("temp")));
        // an explicit y does not change the x default
        let columns = resolve(None, 1, None, Some("1")).unwrap();
        assert_eq!(point_of(&columns, 2, &["9"]), Some((2.0, 9.0)));
    }

    #[test]
    fn columns_are_found_by_name_ignoring_case() {
        let columns = resolve(Some(&["Time", "Temp", "RH"]), 3, Some("time"), Some("rh")).unwrap();
        assert_eq!(columns.names, names(Some("Time"), Some("RH")));
        assert_eq!(columns.y_column, 3);
        assert_eq!(point_of(&columns, 0, &["0", "20", "50"]), Some((0.0, 50.0)));
    }

    #[test]
    fn columns_by_index_take_header_names() {
        let columns = resolve(Some(&["a", "", "c"]), 3, Some("3"), Some("2")).unwrap();
        // an empty header cell gives no name
        assert_eq!(columns.names, names(Some("c"), None));
        assert_eq!(point_of(&columns, 0, &["1", "2", "3"]), Some((3.0, 2.0)));
    }

    #[test]
    fn unknown_column_names_are_an_error() {
        let err = resolve(Some(&["a", "b"]), 2, None, Some("c"))
            .unwrap_err()
            .to_string();
        assert_eq!(err, "t has no column 'c'; available columns: a, b");
        let err = resolve(Some(&["a", "b"]), 2, Some("z"), None)
            .unwrap_err()
            .to_string();
        assert_eq!(err, "t has no column 'z'; available columns: a, b");
    }

    #[test]
    fn column_names_without_a_header_are_an_error() {
        let err = resolve(None, 2, None, Some("temp"))
            .unwrap_err()
            .to_string();
        assert_eq!(
            err,
            "t has no header row, so column 'temp' cannot be found; use a 1-based column number \
             instead"
        );
    }

    #[test]
    fn the_y_column_cannot_be_the_row_number() {
        for width in [1, 2] {
            let err = resolve(None, width, None, Some("index"))
                .unwrap_err()
                .to_string();
            assert_eq!(err, "the y column cannot be 'index'");
        }
    }

    // -- points --

    #[test]
    fn the_row_number_can_be_x() {
        let columns = resolve(Some(&["a", "b"]), 2, Some("index"), None).unwrap();
        assert_eq!(columns.names, names(None, Some("b")));
        assert_eq!(point_of(&columns, 0, &["1", "10"]), Some((0.0, 10.0)));
        assert_eq!(point_of(&columns, 5, &["2", "20"]), Some((5.0, 20.0)));
    }

    #[test]
    fn missing_values_give_no_point() {
        let columns = resolve(None, 2, None, None).unwrap();
        for fields in [
            &["NA", "2"][..],
            &["1", ""],
            &["?", "-"],
            &["null", "1"],
            &["1", "None"],
        ] {
            assert_eq!(point_of(&columns, 0, fields), None, "{fields:?}");
        }
    }

    #[test]
    fn a_missing_field_is_an_error_naming_the_line() {
        let columns = resolve(Some(&["x", "y"]), 2, None, None).unwrap();
        let err = columns
            .point(0, &row(7, &["1"]), "t")
            .unwrap_err()
            .to_string();
        assert_eq!(err, "t:7: y column 'y' is missing (the row has 1 field(s))");

        let columns = resolve(None, 3, Some("3"), None).unwrap();
        let err = columns
            .point(0, &row(2, &["1", "2"]), "t")
            .unwrap_err()
            .to_string();
        assert_eq!(err, "t:2: x column 3 is missing (the row has 2 field(s))");
    }

    #[test]
    fn an_unparsable_value_is_an_error_naming_the_line() {
        let columns = resolve(Some(&["x", "y"]), 2, None, None).unwrap();
        let err = columns
            .point(0, &row(3, &["foo", "3"]), "t")
            .unwrap_err()
            .to_string();
        assert_eq!(
            err,
            "t:3: cannot parse x value 'foo' (column 'x') as a number"
        );
        let columns = resolve(None, 2, None, None).unwrap();
        let err = columns
            .point(0, &row(4, &["1", "bar"]), "t")
            .unwrap_err()
            .to_string();
        assert_eq!(
            err,
            "t:4: cannot parse y value 'bar' (column 2) as a number"
        );
        // a missing x value does not hide a bad y value
        let err = columns
            .point(0, &row(5, &["NA", "bar"]), "t")
            .unwrap_err()
            .to_string();
        assert!(err.starts_with("t:5: cannot parse y value 'bar'"), "{err}");
    }

    // -- streaming matches whole tables --

    /// Reads `content` as a stream would: the columns are resolved as soon as the reader knows
    /// the width, and each row is converted when it arrives.
    fn stream_points(
        content: &str,
        x: Option<&Column>,
        y: Option<&Column>,
        source: &str,
    ) -> Result<Parsed> {
        let mut reader = TableReader::new();
        let mut columns = None;
        let mut parsed = Parsed::default();
        let mut row_number = 0;
        for line in content.lines() {
            let row = reader.push_line(line);
            if columns.is_none()
                && let Some(width) = reader.width()
            {
                columns = Some(Columns::resolve(reader.header(), width, x, y, source)?);
            }
            if let Some(row) = row {
                let columns = columns.as_ref().expect("resolved by the first data row");
                match columns.point(row_number, &row, source)? {
                    Some(point) => parsed.points.push(point),
                    None => parsed.skipped_missing += 1,
                }
                row_number += 1;
            }
        }
        let columns = match columns {
            Some(columns) => columns,
            // nothing but blank lines and comments
            None => Columns::resolve(None, 0, x, y, source)?,
        };
        parsed.names = columns.names;
        parsed.y_column = columns.y_column;
        Ok(parsed)
    }

    #[test]
    fn streaming_gives_the_same_points_as_a_whole_table() {
        let cases: &[(&str, Option<&str>, Option<&str>)] = &[
            ("a,b,c\n1,2,3\n", None, None),
            ("a,b,c\n1,2,3\n", None, Some("c")),
            ("5\n6\n", None, None),
            ("time,temp,humidity\n0,20,50\n1,21,51\n", None, None),
            ("time,temp,humidity\n0,20,50\n1,21,51\n", Some("3"), None),
            (
                "time,temp,humidity\n0,20,50\n1,21,51\n",
                None,
                Some("HUMIDITY"),
            ),
            (
                "time,temp,humidity\n0,20,50\n1,21,51\n",
                Some("index"),
                None,
            ),
            ("0,20\n1,21\n", None, None),
            ("temp\n20\n21\n", None, None),
            ("time,\n0,20\n", None, None),
            ("\"time (s)\" , temp\n0,20\n", None, None),
            ("1,2\n3,4\n", None, None),
            ("# comment\nx,y\n\n1,2\n# more\n3,4\n", None, None),
            ("1\t2\n3\t4\n", None, None),
            ("1   2\n  3 4  \n", None, None),
            (
                "time,temp,\"humidity\"\n0,20,50\n1,21,55\n",
                Some("time"),
                Some("HUMIDITY"),
            ),
            (
                "time,temp,\"humidity\"\n0,20,50\n1,21,55\n",
                Some("1"),
                Some("3"),
            ),
            ("a,b\n1,2\n", None, Some("c")),
            ("1,2\n", None, Some("temp")),
            ("value\n5\n7\n9\n", None, None),
            ("1,10\n2,20\n", Some("index"), Some("2")),
            ("x,y\n1,2\n2,\n3,NA\n4,n/a\n,5\n6,7\n", None, None),
            ("a,b,c\n1,,3\n4,5,6\n", Some("1"), Some("3")),
            ("x,y\n1,2\nfoo,3\n", None, None),
            ("1,2\n3\n", None, None),
            ("1,2\n", None, Some("index")),
            ("\u{feff}1,2\n3,4\n5,6\n", None, None),
            ("\u{feff}time,temp\n0,20\n", None, None),
            ("x;y\n1;2\n3;4\n", None, None),
            ("5\n7\n9\n", None, Some("1")),
            ("t,a,b\n1,2,3\n", None, Some("b")),
            ("x,y\n1,2\n3,4\n", None, None),
            // CRLF line endings, and no lines at all
            ("x,y\r\n1,2\r\n3,4\r\n", None, None),
            ("", None, None),
            ("# nothing\n\n", None, Some("temp")),
        ];
        for &(content, x, y) in cases {
            let x = x.map(|c| Column::parse(c).unwrap());
            let y = y.map(|c| Column::parse(c).unwrap());
            let summary = |parsed: Result<Parsed>| {
                parsed
                    .map(|p| (p.points, p.names, p.y_column, p.skipped_missing))
                    .map_err(|e| e.to_string())
            };
            let whole = summary(Table::parse(content).points(x.as_ref(), y.as_ref(), "data.csv"));
            let streamed = summary(stream_points(content, x.as_ref(), y.as_ref(), "data.csv"));
            assert_eq!(streamed, whole, "{content:?} x={x:?} y={y:?}");
        }
    }
}
