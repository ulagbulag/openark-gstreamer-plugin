# OpenARK GStreamer Plugins

## Requirements

### Development Environment

- [GStreamer Libs](https://gitlab.freedesktop.org/gstreamer/gstreamer-rs/blob/main/README.md#installation)
- [Make](https://www.gnu.org/software/make/manual/make.html)
- [OpenARK](https://github.com/ulagbulag/OpenARK/tree/master/templates/bootstrap)
- [Rust](https://www.rust-lang.org/tools/install)

## Quick Start

In your OpenARK VINE Desktop (aka. `MobileX Station`),

```sh
# Initialize (Install dependencies)
make init

# Build gstreamer plugin
make build

# Configure environment variables
export GST_PLUGIN_PATH="$(pwd)/target/release"

# Configure your test model
MY_VIDEO_MODEL='image'

# Test the video upstreaming (in the background)
gst-launch-1.0 videotestsrc \
    ! jpegenc \
    ! arksink model="${MY_VIDEO_MODEL}" &

# Test the video downstreaming
gst-launch-1.0 arksrc model="${MY_VIDEO_MODEL}" \
    ! jpegdec \
    ! autovideosink
```
