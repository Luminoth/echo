#! /bin/bash

set -e

VERSION=`git rev-parse --short HEAD`

rm -f echo*.zip

mkdir build

cp install.sh build

cargo install --path . --root build/
rm build/.crates*

cd build
zip -r ../echo-$VERSION.zip .

cd ..
rm -rf build
