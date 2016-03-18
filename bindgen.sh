#!/bin/bash
export DYLD_LIBRARY_PATH=/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/

bindgen -static-link hs -match hs -o src/raw.rs \
    `pkg-config libhs --cflags` \
    -I/usr/local/include \
    -I/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/clang/7.0.2/include \
    -I/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX10.11.sdk/usr/include/  \
    /usr/local/Cellar/hyperscan/4.1.0/include/hs/hs.h
