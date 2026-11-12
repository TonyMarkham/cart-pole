use crate::dataset::marker_entry::MarkerEntry;

use serde::Deserialize;

/// A run's markers.json: `{"markers": [...]}`, DatasetWriter's `MarkerList`
/// (unity/packages/simulation.cart-pole.synthetic-data-gen/Runtime/DatasetWriter.cs).
#[derive(Deserialize, Debug)]
pub(crate) struct MarkersFile {
    pub(crate) markers: Vec<MarkerEntry>,
}
