use std::fmt;

macro_rules! ANSI_escape_code {
    ($name:ident, $value:expr) => {
        pub struct $name;

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, $value)
            }
        }
    };
}

// Cursor Functions
ANSI_escape_code!(SetCursorHome, "\x1B[H");
ANSI_escape_code!(CursorVisible, "\x1B[?25h");
ANSI_escape_code!(CursorInvisible, "\x1B[?25l");

// Erase Functions
ANSI_escape_code!(EraseScreen, "\x1B[2J");
ANSI_escape_code!(EraseCurrentLine, "\x1B[2K");
ANSI_escape_code!(EraseLineStartToCursor, "\x1B[1K");

// Common Private Modes
ANSI_escape_code!(EnableAltBuffer, "\x1B[?1049h");
ANSI_escape_code!(DisableAltBuffer, "\x1B[?1049l");

pub struct SetCursorPos(pub u16, pub u16);

impl fmt::Display for SetCursorPos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\x1B[{};{}H", self.0, self.1)
    }
}
