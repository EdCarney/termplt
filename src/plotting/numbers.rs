const NUM_ZERO: &str = "
  000000
0000  0000
00      00
00      00
00  00  00
00  00  00
00      00
00      00
0000  0000
  000000
";
const NUM_ONE: &str = "
    11
  1111
11  11
    11
    11
    11
    11
    11
    11
1111111111
";
const NUM_TWO: &str = "
  222222
2222  2222
22    2222
    2222
    2222
  2222
  2222
2222
2222
2222222222
";
const NUM_THREE: &str = "
3333333333
      3333
    3333
  3333
 33333
    3333
      3333
33      33
3333  3333
  333333
";

pub fn get_bitmap(c: char) -> Vec<Vec<bool>> {
    let str_map = match c {
        '0' => NUM_ZERO,
        '1' => NUM_ONE,
        '2' => NUM_TWO,
        '3' => NUM_THREE,
        _ => panic!("Bitmap not defined for character: '{c}'"),
    };

    // note that bitmaps are written to be human-readable; they need to be modified to be
    // printed; this includes correcting the aspect ratio (every other row element is skipped
    // to ensure the aspect ratio is 1:2); also the string is converted to a vec of i32
    let mut bitmap = str_map
        .lines()
        .map(|row| {
            let mut chars = row
                .to_string()
                .chars()
                .map(|x| if x == ' ' { false } else { true })
                .step_by(2)
                .collect::<Vec<_>>();
            while chars.len() < 5 {
                chars.push(false);
            }
            chars
        })
        .collect::<Vec<_>>();

    // the rows of the bitmap must also be reversed to ensure that lower indices are the bottom
    // of the coordinates
    bitmap.reverse();

    bitmap
}
