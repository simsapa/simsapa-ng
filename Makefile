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
	tokei --type Rust,QML,C++,Javascript,CMake --compact --exclude assets/qml/data/ --exclude assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml --exclude assets/dpd-res/ --exclude backend/src/lookup.rs . | grep -vE '===|---'

simsapa.min.js:
	npx webpack

# qml-test-one:
# 	env QT_QPA_PLATFORM=offscreen qmltestrunner -import ./assets/qml/ -input ./assets/qml/ -functions 'CommonWords::test_clean_stem'

qml-test:
	env QT_QPA_PLATFORM=offscreen qmltestrunner -import ./assets/qml/ -input ./assets/qml/

project-tree:
	tree --gitignore --dirsfirst -I docs/ -I CMakeLists.txt.user -I res/ -I gradle/ -I vendor/ -I dpd-res/ -I fonts/ -I icons/ -I scripts/ -I package-lock.json -I Cargo.lock -o project_tree.txt
