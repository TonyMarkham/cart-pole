//! The dataset the runs make: data.yaml, the one file training is pointed at
//! (`data_yaml`), listing the runs as train or val and naming the classes
//! from the runs' own markers.json (`marker_names`, read as `MarkersFile`).

pub(crate) mod data_yaml;
pub(crate) mod marker_entry;
pub(crate) mod marker_names;
pub(crate) mod markers_file;

pub(crate) use crate::dataset::data_yaml::write_data_yaml;
