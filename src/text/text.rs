use rgb::RGB8;

pub struct Text {
    color: RGB8,
    size: u32,
    text: String,
}

impl Text {
    pub fn new(text: String, color: RGB8, size: u32) -> Text {
        Text { text, color, size }
    }
    pub fn default(text: String) -> Text {
        Self::new(text, colors, size)
    }
}
