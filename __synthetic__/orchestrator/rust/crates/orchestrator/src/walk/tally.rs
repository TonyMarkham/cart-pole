/// How one episode's commands fared, for the log.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub landed: u32,
    pub gobbled: u32,
    pub lost: u32,
}
