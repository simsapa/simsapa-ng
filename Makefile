all: run

build:
	cmake -S . -B ./build/simsapadhammareader/ && cmake --build ./build/simsapadhammareader/

run: build
	./build/simsapadhammareader/simsapadhammareader

db_reset:
	rm ../assets-testing/appdata.sqlite3 && \
	cd rust && \
	diesel migration run && \
	cd ../../assets-testing/ && \
	cat suttas.sql | sqlite3 appdata.sqlite3
