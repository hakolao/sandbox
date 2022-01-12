#!/bin/bash
#stop if there is an error, u complains for undefined vars
set -eu

if [[ ! "$OSTYPE" == "linux-gnu"* ]]; then
  echo "Not on Linux, build not run"
  exit 1
fi

# Assumes we are running thus script on Ubuntu

# Assumes following has been done
# rustup target add x86_64-unknown-linux-gnu
# rustup toolchain install stable-x86_64-unknown-linux-gnu
echo "Building for ubuntu"
cargo build --package sandbox --release --target x86_64-unknown-linux-gnu
BUILD_UBUNTU_SUCCESS=$?


# Ubuntu dist
if [ $BUILD_UBUNTU_SUCCESS -eq 0  ]; then
  mkdir -p ubuntu_build
  mkdir -p ubuntu_build/assets
  cp -r assets/object_images ubuntu_build/assets
  cp target/x86_64-unknown-linux-gnu/release/sandbox ubuntu_build/
  zip -r ubuntu_build.zip ubuntu_build
  rm -rf ubuntu_build
else
  echo "Ubuntu Build failed"
fi

# Assumes following has been done
# rustup target add x86_64-pc-windows-gnu
# rustup toolchain install stable-x86_64-pc-windows-gnu
# sudo apt install mingw-w64
echo "Building for windows"
cargo build --package sandbox --release --target x86_64-pc-windows-gnu
BUILD_WINDOWS_SUCCESS=$?

# Windows dist
if [ $BUILD_WINDOWS_SUCCESS -eq 0  ]; then
  mkdir -p windows_build
  mkdir -p windows_build/assets
  cp -r assets/object_images windows_build/assets
  cp target/x86_64-pc-windows-gnu/release/sandbox.exe windows_build/
  zip -r windows_build.zip windows_build
  rm -rf windows_build
else
  echo "Windows Build failed"
fi

# Assumes following has been done
# rustup target add x86_64-apple-darwin
# rustup toolchain install stable-x86_64-apple-darwin
# See also sandbox/.cargo/config

# If osxcross does not exist, run it ./osxcross_setup.sh
[ ! -d "$(pwd)/osxcross/target/bin" ] && ./osxcross_setup.sh

echo "Building for MacOS"
PATH="$(pwd)/osxcross/target/bin:$PATH" \
CC=o64-clang \
LIBZ_SYS_STATIC=1 \
MAC_OS_BUILD=1 \
cargo build --package sandbox --release --target x86_64-apple-darwin

BUILD_MACOS_SUCCESS=$?
# MacOS build
if [ $BUILD_MACOS_SUCCESS -eq 0  ]; then
  mkdir -p macos_build
  mkdir -p macos_build/assets
  cp -r assets/object_images macos_build/assets
  cp target/x86_64-apple-darwin/release/sandbox macos_build/
  zip -r macos_build.zip macos_build
  rm -rf macos_build
else
  echo "MacOS Build failed"
fi