#!/usr/bin/env bash
cat ./dist/index.html | sed "s@'/@'./@g" | sed 's@"/@"./@g' > ./dist/index2.html
rm ./dist/index.html
mv ./dist/index2.html ./dist/index.html
