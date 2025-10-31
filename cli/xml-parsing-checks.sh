#!/usr/bin/env bash

if [ -f fragments.sqlite3 ]; then
    rm fragments.sqlite3
fi

if [ -f suttas.sqlite3 ]; then
    rm suttas.sqlite3
fi

cargo run -- parse-tipitaka-xml ../../bootstrap-assets-resources/tipitaka-org-vri-cst/tipitaka-xml/romn/s0101m.mul.xml suttas.sqlite3 --fragments-db fragments.sqlite3
cargo run -- parse-tipitaka-xml ../../bootstrap-assets-resources/tipitaka-org-vri-cst/tipitaka-xml/romn/s0201m.mul.xml suttas.sqlite3 --fragments-db fragments.sqlite3

cargo run -- parse-tipitaka-xml ../../bootstrap-assets-resources/tipitaka-org-vri-cst/tipitaka-xml/romn/s0101a.att.xml suttas.sqlite3 --fragments-db fragments.sqlite3
cargo run -- parse-tipitaka-xml ../../bootstrap-assets-resources/tipitaka-org-vri-cst/tipitaka-xml/romn/s0101t.tik.xml suttas.sqlite3 --fragments-db fragments.sqlite3

cargo run -- reconstruct-xml-from-fragments ./fragments.sqlite3 s0101m.mul.xml ./s0101m.mul.xml
cargo run -- reconstruct-xml-from-fragments ./fragments.sqlite3 s0201m.mul.xml ./s0201m.mul.xml

sed -i 's/UTF-16/UTF-8/' ./*.xml

echo "Diff files:"

diff ./tests/data/s0101m.mul.xml ./s0101m.mul.xml
diff ./tests/data/s0201m.mul.xml ./s0201m.mul.xml

echo "Done"
