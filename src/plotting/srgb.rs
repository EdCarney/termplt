//! Mixing colors in linear light, using integer math only so that every platform produces the
//! same bytes.

/// sRGB value to linear light, scaled to `0..=65535`: `round(65535 * linear(i / 255))`, where
/// `linear(c)` is `c / 12.92` up to 0.04045 and `((c + 0.055) / 1.055)^2.4` above. Committed
/// rather than computed so no platform-dependent `powf` rounding reaches the pixels.
#[rustfmt::skip]
const SRGB_TO_LINEAR: [u16; 256] = [
    0, 20, 40, 60, 80, 99, 119, 139, 159, 179, 199, 219,
    241, 264, 288, 313, 340, 367, 396, 427, 458, 491, 526, 562,
    599, 637, 677, 718, 761, 805, 851, 898, 947, 997, 1048, 1101,
    1156, 1212, 1270, 1330, 1391, 1453, 1517, 1583, 1651, 1720, 1790, 1863,
    1937, 2013, 2090, 2170, 2250, 2333, 2418, 2504, 2592, 2681, 2773, 2866,
    2961, 3058, 3157, 3258, 3360, 3464, 3570, 3678, 3788, 3900, 4014, 4129,
    4247, 4366, 4488, 4611, 4736, 4864, 4993, 5124, 5257, 5392, 5530, 5669,
    5810, 5953, 6099, 6246, 6395, 6547, 6700, 6856, 7014, 7174, 7335, 7500,
    7666, 7834, 8004, 8177, 8352, 8528, 8708, 8889, 9072, 9258, 9445, 9635,
    9828, 10022, 10219, 10417, 10619, 10822, 11028, 11235, 11446, 11658, 11873, 12090,
    12309, 12530, 12754, 12980, 13209, 13440, 13673, 13909, 14146, 14387, 14629, 14874,
    15122, 15371, 15623, 15878, 16135, 16394, 16656, 16920, 17187, 17456, 17727, 18001,
    18277, 18556, 18837, 19121, 19407, 19696, 19987, 20281, 20577, 20876, 21177, 21481,
    21787, 22096, 22407, 22721, 23038, 23357, 23678, 24002, 24329, 24658, 24990, 25325,
    25662, 26001, 26344, 26688, 27036, 27386, 27739, 28094, 28452, 28813, 29176, 29542,
    29911, 30282, 30656, 31033, 31412, 31794, 32179, 32567, 32957, 33350, 33745, 34143,
    34544, 34948, 35355, 35764, 36176, 36591, 37008, 37429, 37852, 38278, 38706, 39138,
    39572, 40009, 40449, 40891, 41337, 41785, 42236, 42690, 43147, 43606, 44069, 44534,
    45002, 45473, 45947, 46423, 46903, 47385, 47871, 48359, 48850, 49344, 49841, 50341,
    50844, 51349, 51858, 52369, 52884, 53401, 53921, 54445, 54971, 55500, 56032, 56567,
    57105, 57646, 58190, 58737, 59287, 59840, 60396, 60955, 61517, 62082, 62650, 63221,
    63795, 64372, 64952, 65535,
];

/// The sRGB value whose linear light is nearest `linear`.
fn to_srgb(linear: u32) -> u8 {
    let i = SRGB_TO_LINEAR.partition_point(|&v| u32::from(v) < linear);
    match i {
        0 => 0,
        256 => 255,
        _ => {
            let below = u32::from(SRGB_TO_LINEAR[i - 1]);
            let above = u32::from(SRGB_TO_LINEAR[i]);
            if above - linear < linear - below {
                i as u8
            } else {
                (i - 1) as u8
            }
        }
    }
}

/// One color channel of `fg` drawn over `bg` with `coverage` (0 = none, 255 = all), mixed in
/// linear light.
pub(crate) fn blend(bg: u8, fg: u8, coverage: u8) -> u8 {
    let bg = u32::from(SRGB_TO_LINEAR[usize::from(bg)]);
    let fg = u32::from(SRGB_TO_LINEAR[usize::from(fg)]);
    let a = u32::from(coverage);
    to_srgb((bg * (255 - a) + fg * a + 127) / 255)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_coverage_keeps_the_background_and_full_coverage_paints() {
        for v in [0, 1, 17, 128, 254, 255] {
            assert_eq!(blend(v, 200, 0), v);
            assert_eq!(blend(200, v, 255), v);
        }
    }

    #[test]
    fn partial_coverage_mixes_in_linear_light() {
        // half-covered white on black is 50% light, which is sRGB 188; plain averaging gives
        // 128, only 22% light, so anti-aliased text would look thin
        assert_eq!(blend(0, 255, 128), 188);
        assert_eq!(blend(0, 255, 64), 137);
        assert_eq!(blend(0, 255, 191), 224);
        assert_eq!(blend(255, 0, 128), 187);
    }

    #[test]
    fn table_matches_the_srgb_formula() {
        for (i, &value) in SRGB_TO_LINEAR.iter().enumerate() {
            let c = i as f64 / 255.0;
            let linear = if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            };
            assert!(
                (f64::from(value) - linear * 65535.0).abs() <= 1.0,
                "entry {i}"
            );
        }
        assert!(SRGB_TO_LINEAR.windows(2).all(|w| w[0] < w[1]));
    }
}
