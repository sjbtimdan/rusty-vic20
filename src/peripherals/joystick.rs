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
    fn joystick_right(&mut self, _right: bool);
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
    pub fn step(&self, via1: &mut impl JoystickControl, via2: &mut impl JoystickControl) {
        let (up, down, left, right) = if let Some(d) = self.direction {
            d.bools()
        } else {
            (false, false, false, false)
        };
        via1.joystick_control(up, down, left, self.fire);
        via2.joystick_right(right);
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

    fn noop_right_mock() -> Unimock {
        Unimock::new(
            JoystickControlMock::joystick_right
                .each_call(matching!(false))
                .returns(()),
        )
    }

    #[rstest]
    fn calls_via_correctly_for_nothing_pressed(mut joystick: Joystick) {
        joystick.set_state(JoystickUpdate {
            direction: None,
            fire: false,
        });
        let mut via1 = Unimock::new(
            JoystickControlMock::joystick_control
                .each_call(matching!(false, false, false, false))
                .returns(()),
        );
        let mut via2 = noop_right_mock();
        joystick.step(&mut via1, &mut via2);
        via1.verify();
        via2.verify();
    }

    #[rstest]
    fn calls_via_correctly_for_fire(mut joystick: Joystick) {
        joystick.set_state(JoystickUpdate {
            direction: None,
            fire: true,
        });
        let mut via1 = Unimock::new(
            JoystickControlMock::joystick_control
                .each_call(matching!(false, false, false, true))
                .returns(()),
        );
        let mut via2 = noop_right_mock();
        joystick.step(&mut via1, &mut via2);
        via1.verify();
        via2.verify();
    }

    #[rstest]
    fn calls_via_correctly_for_up_left(mut joystick: Joystick) {
        joystick.set_state(JoystickUpdate {
            direction: Some(JoystickDirection::UpLeft),
            fire: true,
        });
        let mut via1 = Unimock::new(
            JoystickControlMock::joystick_control
                .each_call(matching!(true, false, true, true))
                .returns(()),
        );
        let mut via2 = noop_right_mock();
        joystick.step(&mut via1, &mut via2);
        via1.verify();
        via2.verify();
    }

    #[rstest]
    fn calls_via_correctly_for_right(mut joystick: Joystick) {
        joystick.set_state(JoystickUpdate {
            direction: Some(JoystickDirection::Right),
            fire: false,
        });
        let mut via1 = Unimock::new(
            JoystickControlMock::joystick_control
                .each_call(matching!(false, false, false, false))
                .returns(()),
        );
        let mut via2 = Unimock::new(
            JoystickControlMock::joystick_right
                .each_call(matching!(true))
                .returns(()),
        );
        joystick.step(&mut via1, &mut via2);
        via1.verify();
        via2.verify();
    }

    #[rstest]
    fn calls_via_correctly_for_up_right(mut joystick: Joystick) {
        joystick.set_state(JoystickUpdate {
            direction: Some(JoystickDirection::UpRight),
            fire: false,
        });
        let mut via1 = Unimock::new(
            JoystickControlMock::joystick_control
                .each_call(matching!(true, false, false, false))
                .returns(()),
        );
        let mut via2 = Unimock::new(
            JoystickControlMock::joystick_right
                .each_call(matching!(true))
                .returns(()),
        );
        joystick.step(&mut via1, &mut via2);
        via1.verify();
        via2.verify();
    }

    #[rstest]
    fn calls_via_correctly_for_down_right(mut joystick: Joystick) {
        joystick.set_state(JoystickUpdate {
            direction: Some(JoystickDirection::DownRight),
            fire: true,
        });
        let mut via1 = Unimock::new(
            JoystickControlMock::joystick_control
                .each_call(matching!(false, true, false, true))
                .returns(()),
        );
        let mut via2 = Unimock::new(
            JoystickControlMock::joystick_right
                .each_call(matching!(true))
                .returns(()),
        );
        joystick.step(&mut via1, &mut via2);
        via1.verify();
        via2.verify();
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
