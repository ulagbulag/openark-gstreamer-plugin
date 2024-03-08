#!/bin/bash
# Copyright (c) 2024 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

function install_dependencies_ubuntu() {
    sudo apt-get update && sudo apt-get install --yes \
        build-essential \
        cargo \
        clang \
        gstreamer1.0-alsa \
        gstreamer1.0-gl \
        gstreamer1.0-gtk3 \
        gstreamer1.0-libav \
        gstreamer1.0-pipewire \
        gstreamer1.0-plugins-bad \
        gstreamer1.0-plugins-base \
        gstreamer1.0-plugins-good \
        gstreamer1.0-plugins-ugly \
        gstreamer1.0-pulseaudio \
        gstreamer1.0-qt5 \
        gstreamer1.0-tools \
        gstreamer1.0-vaapi \
        gstreamer1.0-x \
        libclang-dev \
        libges-1.0-dev \
        libgles2-mesa-dev \
        libgstreamer1.0-dev \
        libgstreamer-plugins-bad1.0-dev \
        libgstreamer-plugins-base1.0-dev \
        libgstrtspserver-1.0-dev \
        libgtk2.0-dev \
        libjansson4 \
        libhwloc-dev \
        libprotobuf-dev \
        libprotoc-dev \
        libssl3 \
        libssl-dev \
        libudev-dev \
        libyaml-cpp-dev \
        llvm-dev \
        make \
        mold \
        nasm \
        pkg-config
}

function install_dependencies() {
    # Collect informations
    local os_name=$(cat /etc/os-release | grep -Po '^NAME="\K[\w]*')

    case "${os_name}" in
    Ubuntu)
        install_dependencies_ubuntu ${@:1}
        ;;
    *)
        "Unknown OS: ${os_name}" >&2
        exit 1
        ;;
    esac
}

install_dependencies ${@:1}
