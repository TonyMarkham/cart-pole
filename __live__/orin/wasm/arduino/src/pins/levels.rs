use std::collections::BTreeMap;

/// One reading, as the Arduino sends it: `{"d2":0,"d3":0,...}`, 1 = high.
pub(crate) type Levels = BTreeMap<String, u8>;
