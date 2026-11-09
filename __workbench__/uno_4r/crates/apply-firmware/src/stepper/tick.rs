use ra4m1::{GPT164, GPT165};
use unor4_hal::step::{TickBuilder, TickSetup};

pub(crate) const TICK: TickSetup<GPT164, GPT165> =
    TickBuilder::new().with_timers::<GPT164, GPT165>().build();
