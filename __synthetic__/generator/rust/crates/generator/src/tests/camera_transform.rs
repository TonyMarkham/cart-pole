//! generator.toml's optional camera pose, as the players' `-camera-transform`.

use crate::players::launch::camera_transform_value;

#[test]
fn given_a_camera_pose_when_written_then_it_is_six_numbers_split_by_commas() {
    // given
    let values = [0.0, 1.1, -2.5, 10.0, 0.0, -0.5];

    // when
    let value = camera_transform_value(values);

    // then: the form PlayerController splits on ','
    assert_eq!(value, "0,1.1,-2.5,10,0,-0.5");
}
