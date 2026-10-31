/// Why a command stopped short: the Arduino's `HALT <reason> <count>`.
/// The reasons are calibrate's (`calibrate/src/halt.rs`), in kebab case.
#[derive(Clone, Debug)]
pub struct Halt {
    pub reason: String,
    /// The pulse count where it stopped.
    pub count: i64,
    /// The board's own words for it: its `LOG` line just before the
    /// `HALT`, naming the pin and the direction where it has them.
    pub detail: Option<String>,
}

impl Halt {
    /// Whether the Arduino switched the driver off: a fault or a lost
    /// count, after which the cart's place is no longer known and it must
    /// be homed again. STOP, a switch pressed at the start and an unhomed
    /// request leave the driver holding (calibrate's `frees_cart`).
    pub fn frees_cart(&self) -> bool {
        matches!(
            self.reason.as_str(),
            "driver-fault" | "no-switch" | "same-switch" | "switch-on-the-way" | "wrong-reference" | "unexplained-switch"
        )
    }

    /// The halt in words, for the page and the log: the board's own
    /// words when it sent them, else the Pi's for the reason.
    pub fn describe(&self) -> String {
        if let Some(detail) = &self.detail {
            return detail.clone();
        }
        let count = self.count;
        match self.reason.as_str() {
            "driver-fault" => format!("D8 read low at pulse {count}; it is unconnected with the DM542, so check what is wired to it"),
            "no-switch" => "no switch tripped within the seek's time limit: check the switches' wiring, or whether the motor is stepping at all".to_owned(),
            "pressed-at-start" => "a switch is pressed, so the homing cannot tell which way leads off it: disable the motor, push the cart off it by hand, then press Calibrate".to_owned(),
            "same-switch" => "both ends tripped the same pin: the switches' wiring is wrong; each end needs its own pin".to_owned(),
            "switch-on-the-way" => format!("a switch tripped at pulse {count} during a move that stays clear of both ends: steps were lost, or the switch is faulty"),
            "wrong-reference" => "the return to the reference switch tripped the other one: the cart is not where the count says".to_owned(),
            "stopped" => format!("STOP pressed; stopped at pulse {count}, the driver holding the cart"),
            "not-homed" => "not homed yet: press Calibrate first; to move the cart off a switch before homing, disable the motor and push it by hand".to_owned(),
            "into-switch" => "a switch the cart ran into is still pressed, and the move goes the way the cart was going: move it the other way".to_owned(),
            "unexplained-switch" => "a switch reads pressed, but no move ran into it: the cart was pushed onto it, or its wire is open. The driver is off: check the switch and its wire, push the cart clear by hand, then press Calibrate".to_owned(),
            "bad-command" => "the board did not take the command; its log line before the HALT says why".to_owned(),
            other => format!("halted ({other}) at pulse {count}"),
        }
    }
}
