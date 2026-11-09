use crate::{
    constants::{FRAME_END, FRAME_HEADER_END, FRAME_PROMPT},
    wifi::{datagram::Datagram, frame_step::FrameStep},
};

/// Reads the bridge's frames one byte at a time, as each byte wakes the board,
/// and gives back each whole datagram. The payload is counted, not scanned, so
/// a payload holding `|`, `\r` or `\n` can't end it early (real-open-issues.md,
/// issue 6, step 1). A byte that breaks a frame drops it, and the reader looks
/// for the next prompt.
pub(crate) struct FrameReader {
    step: FrameStep,
    datagram: Datagram,
}

impl FrameReader {
    pub(crate) const fn new() -> Self {
        Self {
            step: FrameStep::Prompt(0),
            datagram: Datagram::new(),
        }
    }

    /// Takes the next byte from the chip, and returns the datagram once its
    /// frame has ended.
    pub(crate) fn push(&mut self, byte: u8) -> Option<Datagram> {
        let next = match self.step {
            FrameStep::Prompt(matched) if byte == FRAME_PROMPT[matched] => {
                if matched + 1 == FRAME_PROMPT.len() {
                    FrameStep::Socket
                } else {
                    FrameStep::Prompt(matched + 1)
                }
            }
            FrameStep::Prompt(_) => FrameStep::restart(byte),

            FrameStep::Socket => match byte {
                b'0'..=b'9' => FrameStep::Socket,
                b',' => FrameStep::Size(None),
                _ => FrameStep::restart(byte),
            },

            FrameStep::Size(size) => match (byte, size) {
                (b'0'..=b'9', _) => size
                    .unwrap_or(0)
                    .checked_mul(10)
                    .and_then(|size| size.checked_add(u32::from(byte - b'0')))
                    .map_or(FrameStep::restart(byte), |size| FrameStep::Size(Some(size))),
                (FRAME_HEADER_END, Some(size)) => {
                    self.datagram.start(size);
                    if size == 0 {
                        FrameStep::End(0)
                    } else {
                        FrameStep::Payload(size)
                    }
                }
                _ => FrameStep::restart(byte),
            },

            FrameStep::Payload(left) => {
                self.datagram.push(byte);
                if left == 1 {
                    FrameStep::End(0)
                } else {
                    FrameStep::Payload(left - 1)
                }
            }

            FrameStep::End(matched) if byte == FRAME_END[matched] => {
                if matched + 1 == FRAME_END.len() {
                    self.step = FrameStep::Prompt(0);
                    return Some(self.datagram);
                }
                FrameStep::End(matched + 1)
            }
            FrameStep::End(_) => FrameStep::restart(byte),
        };
        self.step = next;
        None
    }

    /// Whether a header has ended and its frame hasn't: the payload, or the
    /// line end after it, is still to come.
    pub(crate) fn in_frame(&self) -> bool {
        matches!(self.step, FrameStep::Payload(_) | FrameStep::End(_))
    }

    /// Drops the frame being read, as when its bytes stopped coming.
    pub(crate) fn reset(&mut self) {
        self.step = FrameStep::Prompt(0);
    }
}
