all: run

build:
	cmake -S . -B ./build/simsapadhammareader/ && cmake --build ./build/simsapadhammareader/

run: build
	./build/simsapadhammareader/simsapadhammareader

count-code:
	tokei --type Rust,QML,C++,Javascript,CMake --compact --exclude assets/qml/data/ . | grep -vE '===|---|Total'
