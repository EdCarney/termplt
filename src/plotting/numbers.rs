use super::text::TextStyle;

const CHAR_WIDTH: usize = 5;
const CHAR_HEIGHT: usize = 10;
const NUM_ZERO: &str = "
  000000
0000000000
00      00
00      00
00  00  00
00  00  00
00      00
00      00
0000000000
  000000
";
const NUM_ONE: &str = "
  1111
111111
    11
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
      5555
      5555
5555  5555
  555555
";
const NUM_SIX: &str = "
    666666
  666666  
6666  
666666
66666666
6666  6666
6666  6666
6666  6666
  666666
   6666
";
const NUM_SEVEN: &str = "
7777777777
7777  7777
      7777
    7777
    7777
    77
  7777
  7777
7777
7777
";
const NUM_EIGHT: &str = "
  888888  
8888  8888
8888  8888
8888  8888
  888888
8888  8888
8888  8888
8888  8888
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
const CHAR_SPACE: &str = "
          
          
          
          
          
          
          
          
          
          
";
const CHAR_DECIMAL: &str = "
          
          
          
          
          
          
          
   0000   
   0000   
   0000   
";
const CHAR_DASH: &str = "
          
          
          
          
  000000  
  000000  
          
          
          
          
";
const CHAR_E: &str = "
          
          
          
  eeeeee  
 eee  eee 
eee    eee
eeeeeeeeee
eee       
 eeeeeeee 
  eeeeee  
";

pub fn get_bitmap(c: char, style: &TextStyle) -> Vec<Vec<bool>> {
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
        ' ' => CHAR_SPACE,
        '.' => CHAR_DECIMAL,
        '-' => CHAR_DASH,
        'e' => CHAR_E,
        _ => panic!("Bitmap not defined for character: '{c}'"),
    };

    // note that bitmaps are written to be human-readable; they need to be modified to be
    // printed; this includes correcting the aspect ratio (every other row element is skipped
    // to ensure the aspect ratio is 1:2); also the string is converted to a vec of i32
    let mut bitmap = str_map
        .lines()
        .filter_map(|row| {
            if row.is_empty() {
                return None;
            }

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

            Some(chars)
        })
        .collect::<Vec<_>>();

    // ensure consistent char height
    while bitmap.len() < CHAR_HEIGHT {
        bitmap.push(vec![false; CHAR_WIDTH]);
    }

    // scale the char
    let mut scaled_bitmap = Vec::new();
    for i in 0..CHAR_HEIGHT {
        let mut scaled_row = Vec::new();
        for j in 0..CHAR_WIDTH {
            for _ in 0..style.scale {
                scaled_row.push(bitmap[i][j]);
            }
        }
        for _ in 0..style.scale {
            scaled_bitmap.push(scaled_row.clone());
        }
    }
    let mut bitmap = Vec::new();

    // add padding
    for mut row in scaled_bitmap {
        for _ in 0..style.padding {
            row.insert(0, false);
            row.push(false);
        }
        bitmap.push(row);
    }

    let num_row_items = 2 * style.padding as usize + CHAR_WIDTH * style.scale as usize;
    for _ in 0..style.padding {
        bitmap.insert(0, vec![false; num_row_items]);
        bitmap.push(vec![false; num_row_items]);
    }

    // the rows of the bitmap must also be reversed to ensure that lower indices are the bottom
    // of the coordinates
    bitmap.reverse();

    bitmap
}
