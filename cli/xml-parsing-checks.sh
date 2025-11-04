#!/usr/bin/env bash

set -e

if [ -f fragments.sqlite3 ]; then
    rm fragments.sqlite3
fi

if [ -f suttas.sqlite3 ]; then
    rm suttas.sqlite3
fi

cargo build

for i in "s0101" "s0201"; do
    ./target/debug/simsapa_cli parse-tipitaka-xml ../../bootstrap-assets-resources/tipitaka-org-vri-cst/tipitaka-xml/romn/"$i"m.mul.xml suttas.sqlite3 --fragments-db fragments.sqlite3 --adjust-fragments-tsv assets/adjust-fragments.tsv
    ./target/debug/simsapa_cli parse-tipitaka-xml ../../bootstrap-assets-resources/tipitaka-org-vri-cst/tipitaka-xml/romn/"$i"a.att.xml suttas.sqlite3 --fragments-db fragments.sqlite3 --adjust-fragments-tsv assets/adjust-fragments.tsv
    ./target/debug/simsapa_cli parse-tipitaka-xml ../../bootstrap-assets-resources/tipitaka-org-vri-cst/tipitaka-xml/romn/"$i"t.tik.xml suttas.sqlite3 --fragments-db fragments.sqlite3 --adjust-fragments-tsv assets/adjust-fragments.tsv

    ./target/debug/simsapa_cli reconstruct-xml-from-fragments ./fragments.sqlite3 "$i"m.mul.xml ./"$i"m.mul.xml
    ./target/debug/simsapa_cli reconstruct-xml-from-fragments ./fragments.sqlite3 "$i"a.att.xml ./"$i"a.att.xml
    ./target/debug/simsapa_cli reconstruct-xml-from-fragments ./fragments.sqlite3 "$i"t.tik.xml ./"$i"t.tik.xml
done

sed -i 's/UTF-16/UTF-8/' ./*.xml

for i in "s0101" "s0201"; do
    echo "Diff files $i"
    diff ./tests/data/"$i"m.mul.xml ./"$i"m.mul.xml
    diff ./tests/data/"$i"a.att.xml ./"$i"a.att.xml
    diff ./tests/data/"$i"t.tik.xml ./"$i"t.tik.xml

    rm ./"$i"m.mul.xml
    rm ./"$i"a.att.xml
    rm ./"$i"t.tik.xml
done

echo "Done"
