#!/usr/bin/env bash

mkdir -p /usr/local/bin
pushd /work/src/bin
for file in *.rs; do
    cp "/work/target/release/${file//.rs}" "/usr/local/bin/${file//.rs}"
done
