# Agent Guidelines for Simsapa App

Simsapa is a sutta reader app for reading the Theravāda Tipitaka in Pāli and translated languages, providing Pāli language tools to analyse passages.

## Project Structure

For detailed information about the codebase organization, folder structure, and locations of essential functions, read [PROJECT_MAP.md](./PROJECT_MAP.md).

Keep [PROJECT_MAP.md](./PROJECT_MAP.md) updated as changes are made.

# AI Dev Tasks

Use these files when I request structured feature development using PRDs:

- [create-prd.md](./ai-dev-tasks/create-prd.md)
- [generate-tasks.md](./ai-dev-tasks/generate-tasks.md)
- [process-task-list.md](./ai-dev-tasks/process-task-list.md)

## Build/Test Commands

- **Build:** `make build -B` (CMake + Qt6) or `cmake -S . -B ./build/simsapadhammareader/ && cmake --build ./build/simsapadhammareader/`
- **Run:** `make run` or `./build/simsapadhammareader/simsapadhammareader`
- **TypeScript:** `npx webpack` (builds src-ts/ → assets/js/simsapa.min.js)
- **Sass:** `make sass` or `sass --no-source-map './assets/sass/:./assets/css/'`
- **QML Tests:** `make qml-test` (runs all QML tests with offscreen platform)
- **Rust Tests:** `cd backend && cargo test` (runs all backend tests)
- **Single Test:** `cd backend && cargo test test_name` (replace test_name with specific test function)

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

## Architecture

- Multi-platform Qt6 app
- C++ and Rust using the CXX-Qt library with QML for window layouts and UI widgets
- Rust backend uses SQLite with Diesel ORM
- Rust CXX-Qt bridges define backend functions used in QML elements
