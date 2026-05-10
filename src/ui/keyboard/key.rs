use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Single(char),
    Left,
    Up,
    ClrHome,
    InsDel,
    Ctrl,
    Restore,
    RunStop,
    ShiftLock,
    Return,
    Cbm,
    LeftShift,
    RightShift,
    CrsrUD,
    CrsrLR,
    F1F2,
    F3F4,
    F5F6,
    F7F8,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Single(c) => write!(f, "{c}"),
            Key::Left => write!(f, "LEFT"),
            Key::Up => write!(f, "UP"),
            Key::ClrHome => write!(f, "CLR/HOME"),
            Key::InsDel => write!(f, "INS/DEL"),
            Key::Ctrl => write!(f, "CTRL"),
            Key::Restore => write!(f, "RESTORE"),
            Key::RunStop => write!(f, "RUN/STOP"),
            Key::ShiftLock => write!(f, "SHIFT LOCK"),
            Key::Return => write!(f, "RETURN"),
            Key::Cbm => write!(f, "CBM"),
            Key::LeftShift | Key::RightShift => write!(f, "SHIFT"),
            Key::CrsrUD => write!(f, "CRSR UD"),
            Key::CrsrLR => write!(f, "CRSR LR"),
            Key::F1F2 => write!(f, "F1/F2"),
            Key::F3F4 => write!(f, "F3/F4"),
            Key::F5F6 => write!(f, "F5/F6"),
            Key::F7F8 => write!(f, "F7/F8"),
        }
    }
}
