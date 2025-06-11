all: run

build:
	cmake -S . -B ./build/simsapadhammareader/ && cmake --build ./build/simsapadhammareader/

run: build
	./build/simsapadhammareader/simsapadhammareader

sass:
	sass --no-source-map './assets/sass/:./assets/css/'

sass-watch:
	sass --no-source-map --watch './assets/sass/:./assets/css/'

count-code:
	tokei --type Rust,QML,C++,Javascript,CMake --compact --exclude assets/qml/data/ . | grep -vE '===|---|Total'
