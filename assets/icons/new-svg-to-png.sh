#!/bin/bash

for i in ./new-svg/*.svg; do
    echo "$i"
    inkscape --export-type=png -w 32 -h 32 "$i"
done

mv ./new-svg/*.svg ./svg
mv ./new-svg/*.png ./32x32
