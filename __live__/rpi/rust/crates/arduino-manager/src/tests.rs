use crate::protocol::{Command, Line};

#[test]
fn command_lines() {
    assert_eq!(Command::Move { pulses: -333, rate: 2_000 }.line(), "MOVE -333 2000");
    assert_eq!(Command::Move { pulses: 333, rate: 2_000 }.line(), "MOVE +333 2000");
    assert_eq!(Command::Home.line(), "HOME");
}

#[test]
fn reply_lines() {
    assert!(matches!(Line::parse("OK HOME D2 D3 59828 29914"), Line::Ok { command, values } if command == "HOME" && values.len() == 4));
    assert!(matches!(Line::parse("SWITCH D2 -12"), Line::Switch { pin, count: -12 } if pin == "D2"));
    assert!(matches!(Line::parse("HALT stopped 1500"), Line::Halt(halt) if halt.reason == "stopped" && halt.count == 1500 && !halt.frees_cart()));
    assert!(matches!(Line::parse("HALT no-switch 0"), Line::Halt(halt) if halt.frees_cart()));
    assert!(matches!(Line::parse("READY"), Line::Ready));
    assert!(matches!(Line::parse("LOG stopped: STOP; stopped at pulse 12"), Line::Log(text) if text == "stopped: STOP; stopped at pulse 12"));
    assert!(matches!(Line::parse("HALT unexplained-switch 40"), Line::Halt(halt) if halt.frees_cart()));
    assert!(matches!(Line::parse("HALT into-switch 0"), Line::Halt(halt) if !halt.frees_cart()));
    assert!(matches!(Line::parse("calibrate: joining WiFi network"), Line::Other));
    assert!(matches!(Line::parse("SWITCH D2 lots"), Line::Other));
}
