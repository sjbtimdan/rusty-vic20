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
    pub fn set_state(&mut self, update: JoystickUpdate) {
        if update.direction == self.direction && update.fire == self.fire {
            return;
        };
        info!("Joystick: direction: {:?}, fire: {:?}", update.direction, update.fire);
        self.direction = update.direction;
        self.fire = update.fire;
    }

    pub fn direction(&self) -> Option<JoystickDirection> {
        self.direction
    }

    pub fn fire(&self) -> bool {
        self.fire
    }
}

pub fn make_joystick_channel() -> (SyncSender<JoystickUpdate>, Receiver<JoystickUpdate>) {
    mpsc::sync_channel(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn joystick() -> Joystick {
        Joystick::default()
    }

    #[rstest]
    fn default_has_no_direction_or_fire(joystick: Joystick) {
        assert_eq!(joystick.direction(), None);
        assert!(!joystick.fire());
    }

    #[rstest]
    fn set_state_updates_direction_and_fire(mut joystick: Joystick) {
        joystick.set_state(JoystickUpdate {
            direction: Some(JoystickDirection::Up),
            fire: true,
        });
        assert_eq!(joystick.direction(), Some(JoystickDirection::Up));
        assert!(joystick.fire());
    }

    #[rstest]
    fn set_state_clears_direction_and_fire(mut joystick: Joystick) {
        joystick.set_state(JoystickUpdate {
            direction: Some(JoystickDirection::DownRight),
            fire: true,
        });
        joystick.set_state(JoystickUpdate {
            direction: None,
            fire: false,
        });
        assert_eq!(joystick.direction(), None);
        assert!(!joystick.fire());
    }

    #[rstest]
    fn all_eight_directions_roundtrip(mut joystick: Joystick) {
        let dirs = [
            JoystickDirection::Up,
            JoystickDirection::UpRight,
            JoystickDirection::Right,
            JoystickDirection::DownRight,
            JoystickDirection::Down,
            JoystickDirection::DownLeft,
            JoystickDirection::Left,
            JoystickDirection::UpLeft,
        ];
        for &d in &dirs {
            joystick.set_state(JoystickUpdate {
                direction: Some(d),
                fire: false,
            });
            assert_eq!(joystick.direction(), Some(d));
        }
    }

    #[rstest]
    fn fire_independent_of_direction(mut joystick: Joystick) {
        joystick.set_state(JoystickUpdate {
            direction: None,
            fire: true,
        });
        assert_eq!(joystick.direction(), None);
        assert!(joystick.fire());

        joystick.set_state(JoystickUpdate {
            direction: Some(JoystickDirection::Up),
            fire: false,
        });
        assert_eq!(joystick.direction(), Some(JoystickDirection::Up));
        assert!(!joystick.fire());
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

    #[rstest]
    fn channel_and_model_integration(mut joystick: Joystick) {
        let (tx, rx) = make_joystick_channel();

        tx.send(JoystickUpdate {
            direction: Some(JoystickDirection::Down),
            fire: true,
        })
        .unwrap();
        let update = rx.try_recv().unwrap();
        joystick.set_state(update);

        assert_eq!(joystick.direction(), Some(JoystickDirection::Down));
        assert!(joystick.fire());

        tx.send(JoystickUpdate {
            direction: None,
            fire: false,
        })
        .unwrap();
        let update = rx.try_recv().unwrap();
        joystick.set_state(update);

        assert_eq!(joystick.direction(), None);
        assert!(!joystick.fire());
    }
}
