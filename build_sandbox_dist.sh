#!/bin/bash
#stop if there is an error, u complains for undefined vars
set -eu

cargo build --package sandbox --release
BUILD_SUCCESS=$?

# Ubuntu build
if [ $BUILD_SUCCESS -eq 0  ]; then
  echo "Building for ubuntu"
  mkdir ubuntu_build
  mkdir ubuntu_build/assets
  cp -r assets/object_images ubuntu_build/assets
  cp target/release/sandbox ubuntu_build/
  zip -r ubuntu_build.zip ubuntu_build
else
  echo "Build failed"
fi

