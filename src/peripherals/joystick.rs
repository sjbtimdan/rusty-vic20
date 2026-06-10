use log::info;
use std::sync::mpsc::{self, Receiver, SyncSender};

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

impl JoystickDirection {
    pub fn bools(&self) -> (bool, bool, bool, bool) {
        match self {
            JoystickDirection::Up => (true, false, false, false),
            JoystickDirection::Down => (false, true, false, false),
            JoystickDirection::Left => (false, false, true, false),
            JoystickDirection::Right => (false, false, false, true),
            JoystickDirection::UpLeft => (true, false, true, false),
            JoystickDirection::UpRight => (true, false, false, true),
            JoystickDirection::DownLeft => (false, true, true, false),
            JoystickDirection::DownRight => (false, true, false, true),
        }
    }
}

#[cfg_attr(test, unimock::unimock(api = JoystickControlMock))]
pub trait JoystickControl {
    fn joystick_control(&mut self, up: bool, down: bool, left: bool, fire: bool);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct JoystickUpdate {
    pub direction: Option<JoystickDirection>,
    pub fire: bool,
}

#[derive(Default)]
pub struct Joystick {
    direction: Option<JoystickDirection>,
    fire: bool,
}

impl Joystick {
    pub fn step(&self, via1: &mut impl JoystickControl) {
        let (up, down, left, _) = if let Some(d) = self.direction {
            d.bools()
        } else {
            (false, false, false, false)
        };
        via1.joystick_control(up, down, left, self.fire);
    }

    pub fn set_state(&mut self, update: JoystickUpdate) {
        if update.direction == self.direction && update.fire == self.fire {
            return;
        };
        info!("Joystick: direction: {:?}, fire: {:?}", update.direction, update.fire);
        self.direction = update.direction;
        self.fire = update.fire;
    }
}

pub fn make_joystick_channel() -> (SyncSender<JoystickUpdate>, Receiver<JoystickUpdate>) {
    mpsc::sync_channel(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};
    use unimock::{MockFn, Unimock, matching};

    #[fixture]
    fn joystick() -> Joystick {
        Joystick::default()
    }

    #[rstest]
    fn calls_via_correctly_for_nothing_pressed(mut joystick: Joystick) {
        joystick.set_state(JoystickUpdate {
            direction: None,
            fire: false,
        });
        let mut mock = Unimock::new(
            JoystickControlMock::joystick_control
                .each_call(matching!(false, false, false, false))
                .returns(()),
        );
        joystick.step(&mut mock);
        mock.verify();
    }

    #[rstest]
    fn calls_via_correctly_for_fire(mut joystick: Joystick) {
        joystick.set_state(JoystickUpdate {
            direction: None,
            fire: true,
        });
        let mut via = Unimock::new(
            JoystickControlMock::joystick_control
                .each_call(matching!(false, false, false, true))
                .returns(()),
        );
        joystick.step(&mut via);
        via.verify();
    }

    #[rstest]
    fn calls_via_correctly_for_up_left(mut joystick: Joystick) {
        joystick.set_state(JoystickUpdate {
            direction: Some(JoystickDirection::UpLeft),
            fire: true,
        });
        let mut via = Unimock::new(
            JoystickControlMock::joystick_control
                .each_call(matching!(true, false, true, false))
                .returns(()),
        );
        joystick.step(&mut via);
        via.verify();
    }

    #[test]
    fn channel_roundtrip() {
        let (tx, rx) = make_joystick_channel();
        let update = JoystickUpdate {
            direction: Some(JoystickDirection::UpRight),
            fire: true,
        };
        tx.send(update).unwrap();
        let received = rx.try_recv().unwrap();
        assert_eq!(received.direction, Some(JoystickDirection::UpRight));
        assert!(received.fire);
    }
}
