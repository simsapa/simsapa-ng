# Agent Guidelines for Simsapa App

Simsapa is a sutta reader app for reading the Theravāda Tipitaka in Pāli and translated languages, providing Pāli language tools to analyse passages.

## Architecture

This is a Qt app with QML window layouts, connecting to a Rust back-end using bridge elements with the CXX-Qt library.

- Multi-platform Qt6 app
- C++ and Rust using the CXX-Qt library with QML for window layouts and UI widgets
- Rust backend uses SQLite with Diesel ORM
- Rust CXX-Qt bridges define backend functions used in QML elements

## Project Structure

For detailed information about the codebase organization, folder structure, and locations of essential functions, read [PROJECT_MAP.md](./PROJECT_MAP.md).

Keep [PROJECT_MAP.md](./PROJECT_MAP.md) updated as changes are made.

When working on features, the PRD (Product Requirements Document) files are in
the `tasks/` folder. They often contain the reasoning and logic for existing
features.

Documentation is in the `docs/` folder. Keep it updated for relevant features.

## Specific coding procedures

### New QML components

When you create a new QML component such as `SearchBarInput.qml`, the file has to be added to the `qml_files` list in `bridges/build.rs`.

``` rust
qml_files.push("../assets/qml/SearchBarInput.qml");
```

### New functions on Rust bridge QML components

When adding new functions to Rust bridge QML components such as SuttaBridge, add a corresponding function in the `qmllint` type definition, e.g. SuttaBridge.qml

For example, when implementing the `get_api_key()` method in `sutta_bridge.rs`, add a corresponding function in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` with the correct function signature and a simple return value. The internal logic doesn't have to be repeated, because this is only for the benefit of `qmllint`.

``` qml
function get_api_key(key_name: string): string {
    return 'key_value';
}
```

### New Rust bridges

When you create a new Rust bridge such as `bridges/src/prompt_manager.rs`, it has to be registered as a QmlModule and the Rust file name has to be added to the `rust_files` list in `bridges/build.rs`:

``` rust
.qml_module(QmlModule {
        uri: "com.profoundlabs.simsapa",
        rust_files: &[
                "src/sutta_bridge.rs",
                "src/asset_manager.rs",
                "src/storage_manager.rs",
                "src/prompt_manager.rs",
                "src/api.rs",
        ],
        qml_files: &qml_files,
        ..Default::default()
})
```

`qmllint` requires that the corresponding QML type definition for the Rust bridge has to be created and it should be declared in the `qmldir` file.

```
assets/qml/com/profoundlabs/simsapa/PromptManager.qml
assets/qml/com/profoundlabs/simsapa/qmldir
```

## Build/Test Commands

- **Build:** `make build -B` (CMake + Qt6) or `cmake -S . -B ./build/simsapadhammareader/ && cmake --build ./build/simsapadhammareader/`
- **Run:** `make run` or `./build/simsapadhammareader/simsapadhammareader`
- **TypeScript:** `npx webpack` (builds src-ts/ → assets/js/simsapa.min.js)
- **Sass:** `make sass` or `sass --no-source-map './assets/sass/:./assets/css/'`
- **QML Tests:** `make qml-test` (runs all QML tests with offscreen platform)
- **Rust Tests:** `cd backend && cargo test` (runs all backend tests)
- **Single Test:** `cd backend && cargo test test_name` (replace test_name with specific test function)

### GUI Testing for Agents

**⚠️ Avoid GUI Testing:** As an AI agent, avoid running the GUI application for testing purposes. The WebEngine components require proper process cleanup that may interfere with your terminal session.

If you must test GUI functionality:
- Use `make build -B` to verify compilation only
- Test individual Rust components with `cd backend && cargo test`
- GUI functionality should be tested manually by the user

The command `export QT_QPA_PLATFORM=offscreen && timeout 10 make run` may leave hanging processes that require manual cleanup, which is not suitable for automated agent testing.

## Code Style

Use lowercase snake_case for new functions, variables and id names, E.g:
- `id: next_message, id: message_item, property bool is_collapsed`
- `function export_dialog_accepted()`

- **Rust:** snake_case, standard rustfmt, use `anyhow::Result` for error handling, prefer `tracing` over `println!`

- **TypeScript:** 2-space indents, import * as alias style, use webpack for bundling

- **C++:** lowercase snake_case functions, PascalCase classes, include proper error handling with custom exceptions

- **QML:** PascalCase components, camelCase properties, follow Qt conventions

- **Naming:** Descriptive names, avoid abbreviations, use domain-specific terms (sutta, pali, dhamma)

- **Errors:** Use Result types in Rust, exceptions in C++, proper error propagation throughout stack

