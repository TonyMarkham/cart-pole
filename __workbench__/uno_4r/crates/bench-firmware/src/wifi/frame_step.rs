use crate::constants::FRAME_PROMPT;

/// Where `FrameReader` is within `+UDPRX: <sock>,<len>|<payload>\r\n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FrameStep {
    /// Matching `+UDPRX: `: how much has matched.
    Prompt(usize),
    /// The socket's digits, up to the comma. There is one socket, so the
    /// number isn't kept.
    Socket,
    /// The size's digits, up to `|`: the size so far, None before a digit.
    Size(Option<u32>),
    /// The payload: how many bytes are still to come.
    Payload(u32),
    /// The frame's end, `\r\n`: how much has matched.
    End(usize),
}

impl FrameStep {
    /// Looking for the next frame after `byte` broke this one: `byte` may
    /// itself start the prompt.
    pub(crate) fn restart(byte: u8) -> Self {
        Self::Prompt(usize::from(byte == FRAME_PROMPT[0]))
    }
}
