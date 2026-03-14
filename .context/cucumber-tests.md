# Cucumber Tests Architectural Context

## 1. Overview

The cucumber tests domain provides acceptance testing for the shownotes application using behavior-driven development (BDD). It uses the `cucumber` crate (v0.21) to execute Gherkin `.feature` files against the application's command layer, enabling verification of user-facing workflows with isolated test environments.

## 2. Core Types

### ShownotesWorld (`tests/acceptance/src/lib.rs`)
- **Purpose**: Base World struct that holds all test state and application context for cucumber scenarios
- **Key fields**:
  - `ctx: SystemCtx` - Main application context with services and config
  - `temp_dir: TempDir` - Isolated temporary directory for each scenario
  - `file_paths: HashMap<String, PathBuf>` - Registry of named test files
  - `fake_editor: Arc<FakeEditor>` - Mock editor for testing editor interactions
- **Usage**: Created fresh for each scenario, provides isolated environment

### Feature World Wrappers (`tests/acceptance/tests/*.rs`)
- **Purpose**: Per-feature World structs that compose `ShownotesWorld` with feature-specific state
- **Pattern**: Each test file defines its own `World` struct wrapping `inner: ShownotesWorld`
- **Usage**: Enables feature-specific step definitions while reusing base infrastructure

### FakeEditor (`crates/shownotes/src/feat/external_editor/editors/fake.rs`)
- **Purpose**: Mock implementation of `ExternalEditor` trait for testing without real editors
- **Key methods**:
  - `set_content(&self, content: String)` - Pre-set editor output for next call
  - `open()` - Returns pre-set content, simulating user editing
- **Usage**: Injected into `Services` during test setup to control editor behavior

### Step Definition Functions
- **Purpose**: Rust functions annotated with `#[given]`, `#[when]`, `#[then]` that map Gherkin steps to code
- **Pattern**: Functions take `&mut World` and parsed parameters from step text
- **Usage**: Organized in `pub mod steps` for sharing across features

## 3. Data Flow

```
.feature file (Gherkin text)
    -> Step Definition Function (via cucumber macro matching)
        -> ShownotesWorld state manipulation
            -> Command execution via execute(&ctx, Command::*)
                -> Services layer (with mocked dependencies)
                    -> CommandResult returned
                        -> Assertion in Then step
```

Each scenario:
1. Creates fresh `TempDir` and `SystemCtx` with test services
2. Given steps set up files, symlinks, database state
3. When steps execute commands through `execute(&ctx, Command::*)`
4. Then steps verify results via `CommandResult` or direct state inspection

## 4. Architectural Patterns

- **World Pattern**: Each scenario has isolated state via fresh World instance
- **Composition**: Feature-specific Worlds compose base `ShownotesWorld`
- **Dependency Injection**: `FakeEditor` injected into `Services` to mock external dependencies
- **Command Pattern**: Tests exercise application through `Command` enum, not direct service calls
- **Shared Steps**: Common step definitions extracted to `pub mod steps` for reuse
- **Harness-less Tests**: Each test is a separate binary with `harness = false` and custom `main()`

## 5. Modification Guide

When implementing features in this domain:

1. **Adding a new feature test**:
   - Create `tests/acceptance/tests/features/my_feature.feature` with Gherkin scenarios
   - Create `tests/acceptance/tests/my_feature.rs` with World struct and step definitions
   - Add `[[test]]` entry in `tests/acceptance/Cargo.toml` with `harness = false`
   - Import shared steps from `acceptance::steps` if needed

2. **Adding step definitions**:
   - Use `#[given]`, `#[when]`, `#[then]` with `expr = r#"pattern {string}"#`
   - For complex parsing, use `regex = r"pattern"` instead of `expr`
   - Access World state via `world.inner` if using feature wrapper

3. **Tests should be written** based on user-facing behavior, not implementation details. Focus on what the user sees, not how it's implemented.

4. **Running specific tests**:
   ```bash
   cargo test --package acceptance --test <test_name>
   ```

## 6. Related Contexts

Search for potentially related context files in `.context/` based on the user's request. For example:
- If only adding test utilities, this context may be sufficient
- If adding new commands that need testing, also look for `commands.md` context
- If modifying services that tests mock, also look for `services.md` context
- If adding new application features, multiple contexts (commands + services + cucumber-tests) may be relevant
