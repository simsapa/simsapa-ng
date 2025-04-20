all: run

build:
	cmake -S . -B ./build/simsapadhammareader/ && cmake --build ./build/simsapadhammareader/

run: build
	./build/simsapadhammareader/simsapadhammareader

count-code:
	tokei --type Rust,QML,C++ --exclude assets/qml/data/ . | grep -vE '===|---|Total'

db_reset:
	rm ../assets-testing/appdata.sqlite3 && \
	cd rust && \
	diesel migration run && \
	cd ../../assets-testing/ && \
	cat suttas.sql | sqlite3 appdata.sqlite3
