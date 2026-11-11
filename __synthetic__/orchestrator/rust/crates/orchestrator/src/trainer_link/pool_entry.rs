use crate::{
    orin::orin_reply::PoolRow,
    trainer_link::{pole_message::PoleMessage, sites_message::SitesMessage},
};

use serde::Serialize;

/// 17: one instance's pool to the trainer, the same rows as its Orin's
/// reply (14) without `failed`: a failed pool goes to the shutdown and
/// never reaches the trainer. `time` is the capture's MuJoCo time in s,
/// and `sites` and `pole` its true site positions and pole state, for the
/// reward and the record alone. `fault` is true when the synthetic Arduino
/// went into FAULT after the instance's last send: the entry is then the next
/// step's, its episode over (__documentation__/synthetic-loop-dia.md, *The rail's ends*).
#[derive(Serialize)]
pub struct PoolEntry<'a> {
    pub instance: usize,
    pub id: u32,
    pub time: f64,
    pub pool: &'a [PoolRow],
    pub sites: &'a SitesMessage,
    pub pole: &'a PoleMessage,
    pub fault: bool,
}
