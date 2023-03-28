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

pub mod color {
    // Regular
    pub const RESET: &str = "\x1B[0m";
    pub const RED: &str = "\x1B[31m";
    pub const GREEN: &str = "\x1B[32m";
    pub const YELLOW: &str = "\x1B[33m";
    pub const BLUE: &str = "\x1B[34m";
    pub const MAGENTA: &str = "\x1B[35m";
    pub const CYAN: &str = "\x1B[36m";
    pub const WHITE: &str = "\x1B[37m";
    pub const BLACK: &str = "\x1B[30m";

    pub const BOLD_BLACK: &str = "\x1B[90m";
    pub const BOLD_RED: &str = "\x1B[91m";
    pub const BOLD_GREEN: &str = "\x1B[92m";
    pub const BOLD_YELLOW: &str = "\x1B[93m";
    pub const BOLD_BLUE: &str = "\x1B[94m";
    pub const BOLD_MAGENTA: &str = "\x1B[95m";
    pub const BOLD_CYAN: &str = "\x1B[96m";
    pub const BOLD_WHITE: &str = "\x1B[97m";
    // pub const BOLD_RED: &str = "\x1B[1;31m";
    // pub const BOLD_GREEN: &str = "\x1B[1;32m";
    // pub const BOLD_YELLOW: &str = "\x1B[1;33m";
    // pub const BOLD_BLUE: &str = "\x1B[1;34m";
    // pub const BOLD_MAGENTA: &str = "\x1B[1;35m";
    // pub const BOLD_CYAN: &str = "\x1B[1;36m";
    // pub const BOLD_WHITE: &str = "\x1B[1;37m";
    // pub const BOLD_BLACK: &str = "\x1B[1;30m";
}
