all: run

# Detect platform and set Qt path for macOS
ifeq ($(shell uname),Darwin)
    QT_PATH ?= $(HOME)/Qt/6.10.1/macos
    BUILD_CMD = cmake -S . -B ./build/simsapadhammareader/ -DCMAKE_PREFIX_PATH=$(QT_PATH) && cmake --build ./build/simsapadhammareader/
    RUN_CMD = ./build/simsapadhammareader/simsapadhammareader.app/Contents/MacOS/simsapadhammareader
else
    BUILD_CMD = cmake -S . -B ./build/simsapadhammareader/ && cmake --build ./build/simsapadhammareader/
    RUN_CMD = ./build/simsapadhammareader/simsapadhammareader
endif

build:
	$(BUILD_CMD)

run: build
	$(RUN_CMD)

sass:
	sass --no-source-map './assets/sass/:./assets/css/'

sass-watch:
	sass --no-source-map --watch './assets/sass/:./assets/css/'

count-code:
	tokei --types Rust,QML,C++,TypeScript,Javascript,CMake --compact --exclude assets/qml/data/ --exclude assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml --exclude assets/js/simsapa.min.js --exclude assets/js/vendor/ --exclude assets/pdf-viewer/ --exclude assets/dpd-res/ --exclude backend/src/lookup.rs --exclude "*/tests/" --exclude "tst_*.qml" . | grep -vE '===|---'

count-code-pie:
	tokei -o json --types Rust,QML,C++,TypeScript,Javascript,CMake --exclude assets/qml/data/ --exclude assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml --exclude assets/js/simsapa.min.js --exclude assets/js/vendor/ --exclude assets/pdf-viewer/ --exclude assets/dpd-res/ --exclude backend/src/lookup.rs --exclude "*/tests/" --exclude "tst_*.qml" . | tokei-pie

simsapa.min.js:
	npx webpack

test: rust-test qml-test js-test

# NOTE: Running 'cargo test' in 'bridges/' doesn't compile, but there are no tests there anyway.
# error: linking with `cc` failed

rust-test:
	cd backend && cargo test && cd ../cli && cargo test

js-test:
	npm test

# qml-test-one:
# 	env QT_QPA_PLATFORM=offscreen qmltestrunner -import ./assets/qml/ -input ./assets/qml/ -functions 'CommonWords::test_clean_stem'

qml-test:
	env QT_QPA_PLATFORM=offscreen qmltestrunner -import ./assets/qml/ -input ./assets/qml/

project-tree:
	tree --gitignore --dirsfirst -I docs/ -I CMakeLists.txt.user -I res/ -I gradle/ -I vendor/ -I dpd-res/ -I fonts/ -I icons/ -I scripts/ -I package-lock.json -I Cargo.lock -o project_tree.txt

bootstrap:
	cd cli/ && cargo build && cargo run -- bootstrap --write-new-dotenv

cargo-clean-build:
	cd backend && cargo clean && cargo build && cd ../bridges && cargo clean && cargo build && cd ../cli && cargo clean && cargo build && cd ..

appimage: build
	./build-appimage.sh

appimage-clean:
	rm -rf Simsapa.AppDir appimage-tools Simsapa-*.AppImage

appimage-rebuild: appimage-clean
	./build-appimage.sh --clean --force-download

macos: build
	./build-macos.sh

macos-app: build
	./build-macos.sh --skip-dmg

macos-clean:
	rm -rf ./dist Simsapa-*.dmg

macos-rebuild: macos-clean
	./build-macos.sh --clean
