const CHAR_WIDTH: usize = 5;
const CHAR_HEIGHT: usize = 10;
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
const NUM_FOUR: &str = "
44      44
44      44
44      44
44      44
4444444444
4444444444
        44
        44
        44
        44
";
const NUM_FIVE: &str = "
5555555555
5555      
5555      
55555     
   55555  
     55555
       555
        55
5555    55
  555555
";
const NUM_SIX: &str = "
    66
  6666
6666
66
66666666
6666  6666
66      66
6666  6666
  666666
    66
";
const NUM_SEVEN: &str = "
7777777777
77      77
       777
      777
     777
    777
   777
  777
 777
777
";
const NUM_EIGHT: &str = "
  888888  
8888  8888
88      88
8888  8888
   8888
8888  8888
88      88
88      88
8888  8888
  888888
";
const NUM_NINE: &str = "
  999999  
9999  9999
9999  9999
9999  9999
  999999
    999
   999
  999
 999
999
";

pub fn get_bitmap(c: char) -> Vec<Vec<bool>> {
    let str_map = match c {
        '0' => NUM_ZERO,
        '1' => NUM_ONE,
        '2' => NUM_TWO,
        '3' => NUM_THREE,
        '4' => NUM_FOUR,
        '5' => NUM_FIVE,
        '6' => NUM_SIX,
        '7' => NUM_SEVEN,
        '8' => NUM_EIGHT,
        '9' => NUM_NINE,
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

            // ensure consistent char width
            while chars.len() < CHAR_WIDTH {
                chars.push(false);
            }
            chars
        })
        .collect::<Vec<_>>();

    // ensure consistent char height
    while bitmap.len() < CHAR_HEIGHT {
        bitmap.push(vec![false; CHAR_WIDTH]);
    }

    // the rows of the bitmap must also be reversed to ensure that lower indices are the bottom
    // of the coordinates
    bitmap.reverse();

    bitmap
}
