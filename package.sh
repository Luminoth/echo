#! /bin/bash

VERSION=`git rev-parse --short HEAD`

rm -f package*.zip

mkdir build

cp install.sh build

cargo install --path . --root build/
rm build/.crates*

cd build
zip -r ../package-$VERSION.zip .

cd ..
rm -rf build
