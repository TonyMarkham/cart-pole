use serde::Deserialize;

/// One marker in a run's markers.json, as DatasetWriter writes it
/// (unity/packages/simulation.cart-pole.synthetic-data-gen/Runtime/DatasetWriter.cs,
/// `MarkerEntry`). Example: `{"classId": 2, "label": "Pivot", "id": 920}`.
#[derive(Deserialize, Debug)]
pub(crate) struct MarkerEntry {
    /// The YOLO class: the marker's index in the MarkerManager's list.
    #[serde(rename = "classId")]
    pub(crate) class_id: u32,
    /// The class's name in data.yaml.
    pub(crate) label: String,
}
