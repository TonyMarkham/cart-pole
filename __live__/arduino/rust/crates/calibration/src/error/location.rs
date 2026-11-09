use core::{
    fmt::{self, Display, Formatter},
    panic::Location,
};

/// Where an error was made, printed as `[file:line:column]`. The webserver gets
/// this from the `error-location` crate, which needs the standard library, and
/// the board has none.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ErrorLocation(&'static Location<'static>);

impl From<&'static Location<'static>> for ErrorLocation {
    fn from(location: &'static Location<'static>) -> Self {
        Self(location)
    }
}

impl Display for ErrorLocation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "[{}:{}:{}]",
            self.0.file(),
            self.0.line(),
            self.0.column()
        )
    }
}
