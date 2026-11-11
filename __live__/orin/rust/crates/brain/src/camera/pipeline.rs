use crate::{
    constants::{
        FRAME_HEIGHT, FRAME_WIDTH, GST_CAPS_NVMM, GST_CAPS_RAW, GST_CONVERTER, GST_DO_TIMESTAMP,
        GST_EE_MODE, GST_EXPOSURE_RANGE, GST_FORMAT_GREY, GST_GAIN_RANGE, GST_ISP_GAIN_LOCKED,
        GST_LINK, GST_PAYLOADER, GST_QUIET, GST_SENSOR_ID, GST_SENSOR_MODE, GST_SINK,
        GST_SINK_STDOUT, GST_SOURCE, GST_TNR_MODE,
    },
    settings::camera_settings::CameraSettings,
};

/// `gst-launch-1.0`'s arguments: the Python camera's pipeline
/// (`__live__/orin/python/src/camera.py`), with `gdppay` before the sink so each
/// frame comes with its timestamp, and the sink on standard output, which the
/// brain reads. `-q` keeps GStreamer's own progress messages off that output;
/// the camera software's messages go to standard error, which the brain
/// leaves alone. `do-timestamp` is the decided stamp (`__documentation__/real-loop-dia.md`, *The
/// pool on the rig*). Each element's name and property is a constant in
/// `constants.rs`, with what it is for.
pub(crate) fn arguments(camera: &CameraSettings) -> Vec<String> {
    let exposure = camera.exposure_ns;
    let gain = camera.gain;
    vec![
        GST_QUIET.to_string(),
        GST_SOURCE.to_string(),
        GST_DO_TIMESTAMP.to_string(),
        format!("{GST_SENSOR_ID}={}", camera.sensor_id),
        format!("{GST_SENSOR_MODE}={}", camera.sensor_mode),
        format!("{GST_TNR_MODE}={}", camera.tnr_mode),
        format!("{GST_EE_MODE}={}", camera.ee_mode),
        // Locked, because automatic exposure would smear the swinging pole's end
        // and change the picture from frame to frame.
        format!("{GST_EXPOSURE_RANGE}={exposure} {exposure}"),
        format!("{GST_GAIN_RANGE}={gain} {gain}"),
        GST_ISP_GAIN_LOCKED.to_string(),
        GST_LINK.to_string(),
        format!(
            "{GST_CAPS_NVMM},width={},height={},framerate={}/1",
            camera.width, camera.height, camera.framerate
        ),
        // Out of the camera's GPU memory, halved, one byte per pixel.
        GST_LINK.to_string(),
        GST_CONVERTER.to_string(),
        GST_LINK.to_string(),
        format!(
            "{GST_CAPS_RAW},format={GST_FORMAT_GREY},width={FRAME_WIDTH},height={FRAME_HEIGHT}"
        ),
        GST_LINK.to_string(),
        GST_PAYLOADER.to_string(),
        GST_LINK.to_string(),
        GST_SINK.to_string(),
        GST_SINK_STDOUT.to_string(),
    ]
}
