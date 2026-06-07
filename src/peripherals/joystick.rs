#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JoystickDirection {
    Up,
    UpRight,
    Right,
    DownRight,
    Down,
    DownLeft,
    Left,
    UpLeft,
}

#[derive(Default)]
pub struct Joystick {
    pub direction: Option<JoystickDirection>,
}
