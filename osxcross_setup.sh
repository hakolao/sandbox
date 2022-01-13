#!/bin/bash
# Run with sudo

# https://wapl.es/rust/2019/02/17/rust-cross-compile-linux-to-macos.html
sudo apt-get install libxml2-dev \
  clang \
  gcc \
  g++ \
  zlib1g-dev \
  libmpc-dev \
  libmpfr-dev \
  libgmp-dev

git clone https://github.com/tpoechtrager/osxcross
cd osxcross
wget -nc https://github.com/joseluisq/macosx-sdks/releases/download/10.15/MacOSX10.15.sdk.tar.xz
mv MacOSX10.15.sdk.tar.xz tarballs/

UNATTENDED=yes OSX_VERSION_MIN=10.11 ./build.sh
