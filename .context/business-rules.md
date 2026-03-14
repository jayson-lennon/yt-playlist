# Business Rules Architecture

## Overview

Business rules in this codebase are implemented as **Commands** executed through a central dispatch system using the Command Pattern. The `Command` enum defines all executable operations, and the `execute()` function routes them to domain-specific handlers. This architecture separates presentation (CLI/TUI) from business logic, enabling consistent behavior and easy testing.

## Core Types

### `Command` (`crates/shownotes/src/command/mod.rs`)
- **Purpose**: Enum defining all executable business operations in the application
- **Key variants**: `SourcesAdd`, `NotesAdd`, `NotesSearch`, `PlaylistLoad`, `PlaylistSave`, `MpvLoad`, `LaunchFile`, `GenerateNotes`, `AliasSet`, `UrlAdd`
- **Usage**: Constructed by CLI handlers or TUI action handlers, then passed to `execute()`

### `CommandResult` (`crates/shownotes/src/command/mod.rs`)
- **Purpose**: Type-safe outcome for each command variant
- **Key variants**: Mirror `Command` variants with result data (e.g., `NotesAdded { paths }`, `PlaylistLoaded { playlist_items, virtual_library_items }`)
- **Usage**: Returned by `execute()`, consumed by presentation layer for user feedback

### `SystemCtx` (`crates/shownotes/src/system_ctx.rs`)
- **Purpose**: Container holding all dependencies needed for command execution
- **Key fields**: `services: Services`, `config: Config`, `library_path: CanonicalPath`, `socket_path: String`, `keymap: Keymap`
- **Usage**: Passed to every command handler function

### `Services` (`crates/shownotes/src/services.rs`)
- **Purpose**: Container for all injectable service dependencies
- **Key fields**: `db: NoteDbService`, `storage: PlaylistStorageService`, `sources: SourceDbService`, `editor: ExternalEditorService`, `media: MediaQueryService`, `mpv: MpvClientService`, `file_launcher: FileLauncherService`, `fuzzy_search: FuzzySearchService`
- **Usage**: Accessed via `ctx.services.*` in command handlers

### Backend Traits (in `crates/shownotes/src/feat/`)
- **`NoteDb`**: Note storage operations (`get_note`, `upsert_note`, `search_notes`)
- **`PlaylistStorage`**: Playlist and alias persistence
- **`SourceDb`**: Source URL management
- **`ExternalEditor`**: Editor integration
- **`MediaQuery`**: Media duration analysis
- **`FileLauncher`**: File opening
- **Usage**: Wrapped by Service structs, injected via `Services`

## Data Flow

```
CLI Args / TUI Action
        ↓
    Command enum variant
        ↓
    execute(ctx, command)  [command/mod.rs]
        ↓
    Domain handler function (e.g., notes::add, playlist::load_playlist)
        ↓
    Service layer calls (ctx.services.db, ctx.services.storage, etc.)
        ↓
    Backend implementation (SQLite, system calls, etc.)
        ↓
    CommandResult variant
        ↓
    Presentation layer (CLI output / TUI state update)
```

## Architectural Patterns

1. **Command Pattern**: All business operations are encapsulated as `Command` enum variants with a central `execute()` dispatcher
2. **Dependency Injection**: Services are injected via `SystemCtx`, enabling swappable backends for testing
3. **Service Wrapper Pattern**: Each backend trait has a corresponding Service wrapper (e.g., `NoteDb` -> `NoteDbService`)
4. **Error Handling**: Uses `error_stack::Report<T>` with `change_context()` for error propagation
5. **Async/Await**: All command handlers are async, using tokio runtime

## Modification Guide

When implementing a new business rule:

1. **Analyze the request**: Determine what operation needs to be performed and which services it requires
2. **Add Command variant**: Add a new variant to the `Command` enum in `command/mod.rs`
3. **Add CommandResult variant**: Add a corresponding variant to `CommandResult` with result data
4. **Add dispatch case**: Add a match arm in `execute()` that delegates to a handler function
5. **Implement handler**: Create the handler function in the appropriate sub-module (or create a new one):
   - `command/notes.rs` - Note operations
   - `command/playlist.rs` - Playlist/library management
   - `command/sources.rs` - Source URL management
   - `command/generate.rs` - Show notes generation
   - `command/launcher.rs` - File launching
   - `command/mpv.rs` - MPV player control
6. **Add service if needed**: If new backend capability is required:
   - Create trait in `feat/<domain>/mod.rs`
   - Create Service wrapper
   - Add to `Services` struct
   - Create fake implementation in `test_utils/fakes.rs`
7. **Wire presentation**: Add CLI subcommand in `cli/` or TUI action in `tui/action_handler/`
8. **Write tests**: 
   - Unit tests in the command sub-module (use `NoteTestContext` from `test_utils`)
   - Acceptance tests in `tests/acceptance/tests/` using cucumber

### Example: Adding a new command

```rust
// 1. In command/mod.rs - Add to Command enum
pub enum Command {
    // ... existing variants
    MyNewOperation { param: String },
}

// 2. In command/mod.rs - Add to CommandResult enum
pub enum CommandResult {
    // ... existing variants
    MyNewOperationCompleted { result: String },
}

// 3. In command/mod.rs - Add to execute() match
Command::MyNewOperation { param } => {
    my_module::handle(ctx, &param).await
}

// 4. Create handler in command/my_module.rs
pub async fn handle(
    ctx: &SystemCtx,
    param: &str,
) -> Result<CommandResult, Report<CommandError>> {
    let result = ctx.services.db.some_operation(param).await
        .change_context(CommandError)?;
    Ok(CommandResult::MyNewOperationCompleted { result })
}
```

## Key Files Reference

| Purpose | File Path |
|---------|-----------|
| Command definitions & dispatch | `crates/shownotes/src/command/mod.rs` |
| Services container | `crates/shownotes/src/services.rs` |
| System context | `crates/shownotes/src/system_ctx.rs` |
| Domain models | `crates/shownotes/src/common/domain.rs` |
| Backend traits | `crates/shownotes/src/feat/*/mod.rs` |
| Test utilities | `crates/shownotes/src/test_utils/mod.rs` |
| Fake backends | `crates/shownotes/src/test_utils/fakes.rs` |
| Test services factory | `crates/shownotes/src/test_utils/services.rs` |
| Test context | `crates/shownotes/src/test_utils/context.rs` |
| CLI handlers | `crates/shownotes/src/cli/` |
| TUI action handlers | `crates/shownotes/src/tui/action_handler/` |
| Acceptance tests | `tests/acceptance/tests/` |

## Related Contexts

Search for potentially related context files in `.contexts/` based on the user's request. For example:
- If only adding a CLI flag that uses existing code, only CLI context may be needed
- If adding an entirely new feature, multiple contexts (CLI + business rules + TUI) may be relevant
- If adding a new backend service, check for existing service patterns in `feat/`
