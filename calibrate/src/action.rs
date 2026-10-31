/// What a button on the page asks for. Each button is a form that POSTs
/// to the action's path; a GET of any path is the page itself, so
/// reloading or prefetching the page never moves the cart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Calibrate,
    Centre,
    Stop,
    Enable,
    Disable,
    JogMinus,
    JogPlus,
    Experiment,
}

impl Action {
    pub const ALL: [Action; 8] = [
        Action::Calibrate,
        Action::Centre,
        Action::Stop,
        Action::Enable,
        Action::Disable,
        Action::JogMinus,
        Action::JogPlus,
        Action::Experiment,
    ];

    pub const fn path(self) -> &'static str {
        match self {
            Action::Calibrate => "/calibrate",
            Action::Centre => "/centre",
            Action::Stop => "/stop",
            Action::Enable => "/enable",
            Action::Disable => "/disable",
            Action::JogMinus => "/jog-minus",
            Action::JogPlus => "/jog-plus",
            Action::Experiment => "/experiment",
        }
    }

    /// The button's text, and the action's name in the log.
    pub const fn label(self) -> &'static str {
        match self {
            Action::Calibrate => "Calibrate",
            Action::Centre => "Go to centre",
            Action::Stop => "STOP",
            Action::Enable => "Enable motor",
            Action::Disable => "Disable motor",
            Action::JogMinus => "Jog minus",
            Action::JogPlus => "Jog plus",
            Action::Experiment => "Jump-limit experiment",
        }
    }

    /// The action a request's first line asks for: `POST <path> HTTP/…`.
    /// Anything else, a GET included, asks for none.
    pub fn from_request(head: &[u8]) -> Option<Action> {
        let rest = head.strip_prefix(b"POST ")?;
        let end = rest.iter().position(|&byte| byte == b' ' || byte == b'?').unwrap_or(rest.len());
        let path = &rest[..end];
        Action::ALL.into_iter().find(|action| action.path().as_bytes() == path)
    }
}
