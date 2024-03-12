#!/bin/bash
# Copyright (c) 2024 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

function install_dependencies_ubuntu() {
    apt-get update && apt-get install --yes \
        build-essential \
        cargo \
        clang \
        gcc \
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
        libhwloc-dev \
        libjansson4 \
        libopenblas-dev \
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

    if [ "x${DEEPSTREAM_INSTALL}" = 'xtrue' ]; then
        install_dependencies_ubuntu_deepstream
    fi
}

function install_dependencies_ubuntu_deepstream() {
    local DEEPSTREAM_REFERENCES_REPO_URL="https://github.com/NVIDIA-AI-IOT/deepstream_reference_apps.git"
    local DEEPSTREAM_URL_DOWNLOAD="https://api.ngc.nvidia.com/v2/resources/nvidia/deepstream/versions"
    local DEEPSTREAM_VERSION_MAJOR="6"
    local DEEPSTREAM_VERSION_MINOR="4"
    local DEEPSTREAM_VERSION_PATCH="0"
    local DEEPSTREAM_VERSION_URL="https://raw.githubusercontent.com/NVIDIA-AI-IOT/deepstream_dockers/main/common/version"

    # Get the latest version
    local DEEPSTREAM_VERSION="$(
        curl -s "${DEEPSTREAM_VERSION_URL}" |
            grep -Po '^version\=\K[0-9\.]+$'
    )"

    # Parse the version information
    local DEEPSTREAM_HOME="/opt/nvidia/deepstream/deepstream"
    local DEEPSTREAM_VERSION_MAJOR="${DEEPSTREAM_VERSION_MAJOR:-"$(echo "${DEEPSTREAM_VERSION}" | awk -F '.' '{print $1}')"}"
    local DEEPSTREAM_VERSION_MINOR="${DEEPSTREAM_VERSION_MINOR:-"$(echo "${DEEPSTREAM_VERSION}" | awk -F '.' '{print $2}')"}"
    local DEEPSTREAM_VERSION_PATCH="${DEEPSTREAM_VERSION_PATCH:-"$(echo "${DEEPSTREAM_VERSION}" | awk -F '.' '{print $3}')"}"
    local DEEPSTREAM_VERSION_RELEASE="${DEEPSTREAM_VERSION_MAJOR}.${DEEPSTREAM_VERSION_MINOR}"
    local DEEPSTREAM_VERSION_FULL="${DEEPSTREAM_VERSION_RELEASE}.${DEEPSTREAM_VERSION_PATCH}"
    local DEEPSTREAM_URL_DOWNLOAD="${DEEPSTREAM_URL_DOWNLOAD}/${DEEPSTREAM_VERSION_RELEASE}/files"
    local DEEPSTREAM_FILE_DOWNLOAD="$(
        curl -s "${DEEPSTREAM_URL_DOWNLOAD}" |
            grep -Po "deepstream-${DEEPSTREAM_VERSION_RELEASE}_${DEEPSTREAM_VERSION_FULL}-[0-9]*_$(dpkg --print-architecture).deb" |
            sort |
            tail -n1
    )"

    # Download
    local DEEPSTREAM_FILE="/tmp/deepstream-sdk.deb"
    wget -qO "${DEEPSTREAM_FILE}" "${DEEPSTREAM_URL_DOWNLOAD}/${DEEPSTREAM_FILE_DOWNLOAD}"

    # Decompress the downloaded file
    apt-get install -y "${DEEPSTREAM_FILE}"

    # Install
    pushd "${DEEPSTREAM_HOME}"
    sed -i 's/"rhel"/"rocky"/g' ./*.sh
    ./install.sh
    rm -f *.sh
    popd

    # Download the latest configuration files
    local DEEPSTREAM_MODELS_DIR="${DEEPSTREAM_HOME}/samples/configs/tao_pretrained_models"
    local DEEPSTREAM_SAMPLE_HOME="/opt/deepstream_reference_apps"
    git clone "${DEEPSTREAM_REFERENCES_REPO_URL}" "${DEEPSTREAM_SAMPLE_HOME}"
    pushd "${DEEPSTREAM_SAMPLE_HOME}/deepstream_app_tao_configs/"
    cp -a * "${DEEPSTREAM_MODELS_DIR}"
    popd

    # Download the models
    pushd "${DEEPSTREAM_MODELS_DIR}"
    ./download_models.sh
    popd

    # Change permissions for user-level modification
    chown -R "$(id -u):$(id -g)" "${DEEPSTREAM_HOME}/samples"

    # Cleanup
    rm -rf "${DEEPSTREAM_SAMPLE_HOME}"
    rm -f "${DEEPSTREAM_FILE}"
}

function install_dependencies() {
    # Collect informations
    local os_name=$(cat /etc/os-release | grep -Po '^NAME="\K[\w]*')

    case "${os_name}" in
    Ubuntu)
        install_dependencies_ubuntu ${@:1}
        ;;
    *)
        echo "Unknown OS: ${os_name}" >&2
        exit 1
        ;;
    esac
}

install_dependencies ${@:1}
