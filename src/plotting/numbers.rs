use super::text::TextStyle;

pub const CHAR_WIDTH: usize = 10;
pub const CHAR_HEIGHT: usize = 11;
const NUM_ZERO: &str = "
  000000
 00000000 
00      00
00      00
00      00
00  00  00
00      00
00      00
00      00
 00000000 
  000000
";
const NUM_ONE: &str = "
  1111
 11 11
11  11
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
 22222222
2222  2222
222    222
      222
     222
    222
   222
  222
 222
222
2222222222
";
const NUM_THREE: &str = "
3333333333
      3333
     333
    333
   333
    333 
     333
      333
333    333
 333   333
  3333333
";
const NUM_FOUR: &str = "
44      44
44      44
44      44
44      44
44      44
4444444444
        44
        44
        44
        44
        44
";
const NUM_FIVE: &str = "
5555555555
55
55
555555
    5555
      5555
       555
        55
        55
55      55
  555555
";
const NUM_SIX: &str = "
       666
      666
     666
    666
   6666
 66666666
6666  6666
666    666
666    666
 666  666
  666666
";
const NUM_SEVEN: &str = "
7777777777
77     777
       777
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
888    888
888    888
888    888
  888888
888    888
888    888
888    888
888    888
888    888
  888888
";
const NUM_NINE: &str = "
  999999  
999    999
999    999
999    999
999    999
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
          
          
          
          
          
";
const CHAR_E: &str = "
          
          
          
          
  eeeeee  
 ee    ee 
ee      ee
eeeeeeeeee
ee        
 ee    ee
  eeeeee
";

/// Drawn for characters without a bitmap.
const CHAR_UNKNOWN: &str = "
##########
##      ##
##      ##
##      ##
##      ##
##      ##
##      ##
##      ##
##      ##
##      ##
##########
";

/// Returns the bitmap for `c`. Characters without a bitmap are drawn as a hollow box.
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
        _ => CHAR_UNKNOWN,
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
                .map(|x| x != ' ')
                //.step_by(2)
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
    for row in bitmap.iter().take(CHAR_HEIGHT) {
        let mut scaled_row = Vec::new();
        for &px in row.iter().take(CHAR_WIDTH) {
            for _ in 0..style.scale() {
                scaled_row.push(px);
            }
        }
        for _ in 0..style.scale() {
            scaled_bitmap.push(scaled_row.clone());
        }
    }
    let mut bitmap = Vec::new();

    // add padding
    for mut row in scaled_bitmap {
        for _ in 0..style.padding() {
            row.insert(0, false);
            row.push(false);
        }
        bitmap.push(row);
    }

    let num_row_items = 2 * style.padding() + CHAR_WIDTH * style.scale();
    for _ in 0..style.padding() {
        bitmap.insert(0, vec![false; num_row_items]);
        bitmap.push(vec![false; num_row_items]);
    }

    // the rows of the bitmap must also be reversed to ensure that lower indices are the bottom
    // of the coordinates
    bitmap.reverse();

    bitmap
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_character_uses_placeholder() {
        let style = TextStyle::default();
        assert_eq!(get_bitmap('N', &style), get_bitmap('?', &style));
        assert_ne!(get_bitmap('N', &style), get_bitmap(' ', &style));
    }
}
