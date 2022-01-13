#!/bin/bash
# stop if there is an error, u complains for undefined vars
set -eu

APP_NAME=sandbox
UBUNTU=x86_64-unknown-linux-gnu
WINDOWS=x86_64-pc-windows-gnu
MACOS=x86_64-apple-darwin
BUILD_DIR=build

if [[ ! "$OSTYPE" == "linux-gnu"* ]]; then
  echo "Not on Linux, build not run"
  exit 1
fi

echo "Building for ubuntu"
rustup target add $UBUNTU
rustup toolchain install stable-$UBUNTU
cargo build --package sandbox --release --target $UBUNTU
BUILD_UBUNTU_SUCCESS=$?

# Ubuntu dist
if [ $BUILD_UBUNTU_SUCCESS -eq 0  ]; then
  mkdir -p $BUILD_DIR/ubuntu_build
  mkdir -p $BUILD_DIR/ubuntu_build/assets
  cp -r assets/object_images $BUILD_DIR/ubuntu_build/assets
  cp target/$UBUNTU/release/$APP_NAME $BUILD_DIR/ubuntu_build/
  cd $BUILD_DIR
  zip -r sandbox_ubuntu.zip ubuntu_build
  cd ../
  rm -rf $BUILD_DIR/ubuntu_build
else
  echo "Ubuntu Build failed"
fi

# Assumes following has been done
rustup target add $WINDOWS
rustup toolchain install stable-$WINDOWS
sudo apt install mingw-w64
echo "Building for windows"
cargo build --package sandbox --release --target $WINDOWS
BUILD_WINDOWS_SUCCESS=$?

# Windows dist
if [ $BUILD_WINDOWS_SUCCESS -eq 0  ]; then
  mkdir -p $BUILD_DIR/windows_build
  mkdir -p $BUILD_DIR/windows_build/assets
  cp -r assets/object_images $BUILD_DIR/windows_build/assets
  cp target/$WINDOWS/release/$APP_NAME.exe $BUILD_DIR/windows_build/
  cd $BUILD_DIR
  zip -r sandbox_windows.zip windows_build
  cd ../
  rm -rf $BUILD_DIR/windows_build
else
  echo "Windows Build failed"
fi

# If osxcross does not exist, run it ./osxcross_setup.sh
[ ! -d "$(pwd)/osxcross/target/bin" ] && ./osxcross_setup.sh

echo "Building for MacOS"
rustup target add $MACOS
rustup toolchain install stable-$MACOS

PATH="$(pwd)/osxcross/target/bin:$PATH" \
  CC=o64-clang \
  LIBZ_SYS_STATIC=1 \
  MAC_OS_BUILD=1 \
  C_INCLUDE_PATH=$(pwd)/osxcross/target/SDK/MacOSX10.15.sdk/usr/include
  cargo build --package sandbox --release --target $MACOS

BUILD_MACOS_SUCCESS=$?
# MacOS build
if [ $BUILD_MACOS_SUCCESS -eq 0  ]; then
  cp -r macos_build_assets/$APP_NAME.app $BUILD_DIR/
  mkdir -p $BUILD_DIR/$APP_NAME.app/assets
  cp -r assets/object_images $BUILD_DIR/$APP_NAME.app/assets
  cp target/$MACOS/release/$APP_NAME $BUILD_DIR/$APP_NAME.app/
  cd $BUILD_DIR
  zip -r sandbox_macos.zip $APP_NAME.app
  cd ../
  rm -rf $BUILD_DIR/$APP_NAME.app
else
  echo "MacOS Build failed"
fi