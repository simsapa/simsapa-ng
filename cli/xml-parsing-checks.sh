#!/usr/bin/env bash

set -e

if [ -f fragments.sqlite3 ]; then
    rm fragments.sqlite3
fi

if [ -f suttas.sqlite3 ]; then
    rm suttas.sqlite3
fi

cargo build

for i in s0101m.mul.xml s0101a.att.xml s0101t.tik.xml s0201m.mul.xml s0201a.att.xml s0201t.tik.xml s0301m.mul.xml s0301a.att.xml s0301t.tik.xml s0402m2.mul.xml s0402a.att.xml s0402t.tik.xml; do
    ./target/debug/simsapa_cli parse-tipitaka-xml ../../bootstrap-assets-resources/tipitaka-org-vri-cst/tipitaka-xml/romn/"$i" suttas.sqlite3 --fragments-db fragments.sqlite3 --adjust-fragments-tsv assets/adjust-fragments.tsv

    ./target/debug/simsapa_cli reconstruct-xml-from-fragments ./fragments.sqlite3 "$i" ./"$i"
done

sed -i 's/UTF-16/UTF-8/' ./*.xml

for i in s0101m.mul.xml s0101a.att.xml s0101t.tik.xml s0201m.mul.xml s0201a.att.xml s0201t.tik.xml s0301m.mul.xml s0301a.att.xml s0301t.tik.xml s0402m2.mul.xml s0402a.att.xml s0402t.tik.xml; do
    echo "Diff files $i"
    diff ./tests/data/"$i" ./"$i"
    rm ./"$i"
done

echo "Done"
