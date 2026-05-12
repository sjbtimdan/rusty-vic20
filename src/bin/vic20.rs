use rusty_vic20::controller::Vic20Controller;

fn main() {
    env_logger::init();
    let mut controller = Vic20Controller::default();
    controller.run();
}
