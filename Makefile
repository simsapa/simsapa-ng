all: run

build:
	cmake -S . -B build && cmake --build build

run: build
	./build/example_simsapa

db_reset:
	rm ../assets-testing/appdata.sqlite3 && \
	cd rust && \
	diesel migration run && \
	cd ../../assets-testing/ && \
	cat suttas.sql | sqlite3 appdata.sqlite3
