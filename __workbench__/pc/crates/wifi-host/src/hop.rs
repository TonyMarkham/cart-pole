use crate::ping::Ping;

/// One datagram's result.
pub(crate) struct Hop {
    pub(crate) ping: Ping,
    /// The hop, µs: the R4's stamp in the PC's time less the PC's send. None
    /// for a lost datagram.
    pub(crate) hop_us: Option<f64>,
}
