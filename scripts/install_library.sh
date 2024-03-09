#!/bin/bash
# Copyright (c) 2024 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

function install_library() {
    # Collect informations
    local gst_version='1.0' # NOTE: hardcoded!
    local gst_lib_home="/usr/lib/$(gcc -dumpmachine)/gstreamer-${gst_version}/"

    # test home
    if [ ! -d "${gst_lib_home}" ]; then
        echo "Cannot find GStreamer library directory: ${gst_lib_home}" >&2
        exit 1
    fi

    sudo install -m755 './target/release/libgsark.so' "${gst_lib_home}"
}

install_library ${@:1}
