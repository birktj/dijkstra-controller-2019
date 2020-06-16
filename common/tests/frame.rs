use common::*;

#[test]
fn read_write() {
    for state in &[
        MotorState::Off,
        MotorState::Idle(54),
        MotorState::Fwd(32),
        MotorState::Rev(154),
    ] {
        for dir in &[213, 4355, 17000, 33556, 50000] {
            let frame = Frame {
                id: 1,
                motor_state: *state,
                motor_direction: MotorDirection::from_pot(*dir),
            };

            let mut buf = [0; 12];

            frame.write(&mut buf);

            assert_eq!(Some(frame), Frame::read(&buf));
        }
    }
}
