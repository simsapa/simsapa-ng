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
	tokei --type Rust,QML,C++,TypeScript,Javascript,CMake --compact --exclude assets/qml/data/ --exclude assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml --exclude assets/js/simsapa.min.js --exclude assets/js/vendor/ --exclude assets/dpd-res/ --exclude backend/src/lookup.rs . | grep -vE '===|---'

simsapa.min.js:
	npx webpack

test: rust-test qml-test

rust-test:
	cd backend && cargo test && cd ../bridges && cargo test && cd ../cli && cargo test

# qml-test-one:
# 	env QT_QPA_PLATFORM=offscreen qmltestrunner -import ./assets/qml/ -input ./assets/qml/ -functions 'CommonWords::test_clean_stem'

qml-test:
	env QT_QPA_PLATFORM=offscreen qmltestrunner -import ./assets/qml/ -input ./assets/qml/

project-tree:
	tree --gitignore --dirsfirst -I docs/ -I CMakeLists.txt.user -I res/ -I gradle/ -I vendor/ -I dpd-res/ -I fonts/ -I icons/ -I scripts/ -I package-lock.json -I Cargo.lock -o project_tree.txt

bootstrap:
	cd cli/ && cargo build && cargo run -- bootstrap --write-new-dotenv

appimage: build
	./build-appimage.sh

appimage-clean:
	rm -rf Simsapa.AppDir appimage-tools Simsapa-*.AppImage

appimage-rebuild: appimage-clean
	./build-appimage.sh --clean --force-download
