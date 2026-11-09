/// One datagram: its 3 bytes, when the PC sent it (ns since the run
/// started), and the R4's stamp of when it had it whole, or None if no
/// report came: lost.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ping {
    pub(crate) payload: [u8; 3],
    pub(crate) sent_ns: f64,
    pub(crate) stamp: Option<u32>,
}
