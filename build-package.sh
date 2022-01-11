#! /bin/bash

set -e

VERSION=`git rev-parse --short HEAD`

rm -rf build/
rm -f echo*.zip

mkdir build

cp install.sh build

cargo install --target x86_64-unknown-linux-musl --path . --root build/
rm build/.crates*

cd build
zip -r ../echo-$VERSION.zip .

cd ..
rm -rf build
