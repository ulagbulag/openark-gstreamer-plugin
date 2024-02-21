use gst::PadTemplate;
use gst_video::{VideoCapsBuilder, VideoFormat};

pub mod sink {
    use super::*;

    pub const SUPPORTED_VIDEO_FORMATS: [VideoFormat; 2] = [VideoFormat::Rgb, VideoFormat::Rgbx];

    pub fn template() -> PadTemplate {
        let caps = VideoCapsBuilder::new()
            .format_list(SUPPORTED_VIDEO_FORMATS)
            .build();

        gst::PadTemplate::new(
            "sink",
            gst::PadDirection::Sink,
            gst::PadPresence::Always,
            &caps,
        )
        .unwrap()
    }
}

pub mod src {
    use super::*;

    pub const SUPPORTED_VIDEO_FORMATS: [VideoFormat; 2] = [VideoFormat::Rgb, VideoFormat::Rgbx];

    pub fn template() -> PadTemplate {
        let caps = VideoCapsBuilder::new()
            .format_list(SUPPORTED_VIDEO_FORMATS)
            .build();

        gst::PadTemplate::new(
            "src",
            gst::PadDirection::Src,
            gst::PadPresence::Always,
            &caps,
        )
        .unwrap()
    }
}
