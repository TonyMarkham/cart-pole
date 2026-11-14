//! The generator's unit tests, one file per behaviour: reading a set from its
//! JSON (`set_json`), laying a set file over the master (`merge`), the rules
//! a pool must meet (`pool_rules`), the players' window grid
//! (`player_grid`), each player's port and window (`placement`), which
//! player replies count as a finished run
//! (`check_done`), the class names in a run's markers.json (`marker_names`)
//! and data.yaml (`data_yaml`), and the players' camera pose argument
//! (`camera_transform`). The set they share is in `fixture`; a test that
//! writes files gets its own folder from `temp_dir`.

mod camera_transform;
mod check_done;
mod data_yaml;
mod fixture;
mod marker_names;
mod merge;
mod placement;
mod player_grid;
mod pool_rules;
mod set_json;
mod temp_dir;
