use ra4m1::GPT321;
use unor4_hal::step::{CounterBuilder, CounterSetup};

pub(crate) const COUNTER: CounterSetup<GPT321> =
    CounterBuilder::new().with_timer::<GPT321>().build();
