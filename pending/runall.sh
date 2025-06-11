#! /usr/bin/env bash

for file in $(ls pending/*.typ); do
  cargo run -- compile $file
done
