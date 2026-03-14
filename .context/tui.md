# TUI Architecture

## Overview

The TUI (Terminal User Interface) layer manages all interactive terminal rendering and user input handling. It implements a component-based architecture where UI elements implement `Component` for input handling and `Render` for drawing, with actions flowing from keymap lookups through `TuiAction` to `Command` execution.

## Core Types

### `TuiAction` (`tui/tui_action.rs`)

- **Purpose**: Represents user intent decoupled from key input. All keyboard actions map to `TuiAction` variants.
- **Key variants**: `Quit`, `Save`, `MoveUp`, `MoveDown`, `SwitchPane`, `Rename`, `LaunchFile`, `LaunchMpv`, `AddUrl`, `Delete`, `GenerateShowNotes(ShowNoteKind)`, etc.
- **Usage**: Produced by `GlobalKeyHandler` after keymap lookup, dispatched via `action_handler::dispatch()` to handler functions.

### `TuiActionResponse` (`tui/tui_action.rs`)

- **Purpose**: Signals whether the application should continue or quit after processing an action.
- **Key variants**: `Continue`, `ShouldQuit`
- **Usage**: Returned by action handlers to control app lifecycle.

### `Component` trait (`tui/component.rs`)

- **Purpose**: Defines how UI components handle keyboard input with event bubbling.
- **Key methods**:
  - `is_active(&self) -> bool` - Returns true if component should receive events (default: false)
  - `handle_key(&mut self, key: KeyEvent) -> EventResult` - Process key, return `Consumed` or `Ignored`
  - `handle_key_with_context(&mut self, key: KeyEvent, ctx: &ComponentContext) -> EventResult` - Override for keymap access
- **Usage**: Implement for any component that needs keyboard input. Return `EventResult::Ignored` to let events bubble to next handler.

### `Render` trait (`tui/render.rs`)

- **Purpose**: Defines how UI components draw themselves to the terminal.
- **Key methods**:
  - `should_render(&self, ctx: &RenderContext) -> bool` - Returns true if component should draw
  - `render(&self, ctx: &mut RenderContext)` - Draw the component to `ctx.frame`
  - `try_render(&self, ctx: &mut RenderContext)` - Default impl: checks `should_render` then calls `render`
- **Usage**: Implement for all visible components. Use `ctx.area` for positioning, `ctx.frame` for drawing widgets.

### `RenderContext` (`tui/render.rs`)

- **Purpose**: Provides all dependencies needed during rendering.
- **Key fields**: `frame: &'a mut Frame<'frame>`, `area: Rect`, `keymap: &'a Keymap`, `services: &'a Services`, `tui_state: &'a TuiState`
- **Usage**: Created in `tui::render()`, passed to all `Render::try_render()` calls. Area is modified by `AreaRender::to()`.

### `ComponentContext` (`tui/component.rs`)

- **Purpose**: Provides dependencies needed during key handling.
- **Key fields**: `keymap: &'a Keymap`, `focused_pane: Pane`
- **Usage**: Created in `App::handle_event()`, passed to `TuiState::handle_key()`.

### `TuiState` (`tui/state.rs`)

- **Purpose**: Central container for all mutable UI state.
- **Key fields**: `playlist_pane`, `library_pane`, `focused_pane`, `status_bar`, `rename`, `url_input`, `global_handler`, `error_popup`, `display_mode`
- **Key methods**: `handle_key()`, `set_status()`, `show_error()`, `selected_playlist_item()`, `start_filter()`, `start_rename()`
- **Usage**: Owned by `App`, modified by action handlers and component key handlers.

### `TuiActionCtx` (`tui/action_handler/mod.rs`)

- **Purpose**: Context passed to action handlers for state mutation and command execution.
- **Key fields**: `tui_state: &'a mut TuiState`, `fork: &'a mut Fork`, `ctx: &'a SystemCtx`
- **Key methods**: `execute(&mut self, command: Command) -> Result<CommandResult, Report<CommandError>>`
- **Usage**: Created in `App::handle_event()` after `TuiAction` is produced, passed to `dispatch()`.

### `GlobalKeyHandler` (`tui/global_key_handler.rs`)

- **Purpose**: Orchestrates keymap lookups and which-key popup display.
- **Key methods**: `take_action() -> Option<TuiAction>`, `toggle_help()`, `is_showing_help()`
- **Usage**: Implements both `Component` and `Render`. Handles prefix keys (e.g., "g" for "gm") via `handle_key_with_context()`.

### `EventResult` (`tui/event.rs`)

- **Purpose**: Controls event bubbling through component hierarchy.
- **Key variants**: `Consumed` (stop propagation), `Ignored` (pass to next handler)
- **Usage**: Returned by `Component::handle_key()` implementations.

## Data Flow

### Input Flow (Key to Action to Command)

```
User presses key
      ↓
App::handle_event(Event::Key(key))
      ↓
TuiState::handle_key(key, &ComponentContext)
      ↓
[Priority order - first active wins:]
  1. ErrorPopup (if active) → dismisses on any key
  2. Rename (if active) → text input
  3. UrlInput (if active) → URL text input
  4. Focused Pane (Playlist/Library) → j/k navigation, filter
  5. GlobalKeyHandler → keymap lookup or which-key
      ↓
GlobalKeyHandler::take_action() → Option<TuiAction>
      ↓
TuiActionCtx created
      ↓
execute_tui_action(&mut TuiActionCtx, TuiAction)
      ↓
action_handler::dispatch() → handler function
      ↓
Handler may:
  - Modify TuiState directly (e.g., MoveUp)
  - Execute Command via ctx.execute() (e.g., Save)
  - Set Fork flags for external processes (e.g., Notes)
      ↓
Return TuiActionResponse (Continue or ShouldQuit)
```

### Render Flow

```
tui::render(frame, state, keymap, services)
      ↓
Create RenderContext { frame, area, keymap, services, tui_state }
      ↓
Layout: split into panes area + status area
      ↓
Split panes area: left (playlist) | right (library)
      ↓
AreaRender::to(area).try_render(&mut ctx, &component) for each:
  - playlist_pane (always renders)
  - library_pane (always renders)
  - status_bar (always renders)
  - rename (renders when active)
  - url_input (renders when active)
  - global_handler (renders which-key when active)
  - error_popup (renders when active)
```

## Architectural Patterns

1. **Component/Render Separation**: Input handling (`Component`) and drawing (`Render`) are separate traits, allowing flexible composition.

2. **Event Bubbling**: Components return `EventResult::Ignored` to pass events to the next handler in priority order.

3. **Context Objects**: `ComponentContext`, `RenderContext`, `TuiActionCtx` encapsulate dependencies and reduce coupling.

4. **Command Pattern**: `TuiAction` represents intent, `Command` represents executable operations. Not all actions become commands.

5. **Fork/Deferred Execution**: Actions requiring external processes (editors) set flags in `Fork` struct, executed after TUI suspends.

6. **Which-Key Pattern**: Multi-key sequences (e.g., "gm" for LaunchMpv) show available bindings via `WhichKey` component.

7. **Active State Pattern**: Components use `is_active()` to control event routing (e.g., `Rename` only consumes keys when `active == true`).

## Modification Guide

When implementing features in this domain:

1. **Adding a new TuiAction**:
   - Add variant to `TuiAction` enum in `tui/tui_action.rs`
   - Add handler function in appropriate `action_handler/*.rs` module
   - Add match arm in `action_handler/mod.rs::dispatch()`
   - Add keybinding in `feat/keymap/map.rs` if needed

2. **Adding a new component**:
   - Create struct with state fields
   - Implement `Component` trait with `is_active()` and `handle_key()`
   - Implement `Render` trait with `should_render()` and `render()`
   - Add field to `TuiState` in `tui/state.rs`
   - Add to render order in `tui/mod.rs::render()`
   - Add to event priority in `TuiState::handle_key()`

3. **Adding a popup/modal**:
   - Create component implementing both traits
   - Set `should_render()` to return `self.is_active()`
   - Add early-return check in `TuiState::handle_key()` for priority
   - Use `Clear` widget before rendering to clear area

4. **Tests should be written based on the user's request**:
   - Unit tests for component behavior go in component's module
   - Integration tests for action handlers go in `app.rs` using `TestAppBuilder`
   - Test event flow with `handle_event(key_event('x'))`
   - Test actions directly with `execute_action(&mut app, TuiAction::Foo)`

## Related Contexts

Search for potentially related context files in `.context/` based on the user's request. For example:
- If only adding a CLI flag that uses existing code, only CLI context may be needed
- If adding an entirely new feature, multiple contexts (CLI + business rules + TUI) may be relevant
- If modifying data persistence, see `services.md` for service patterns
- If modifying keybindings, see `feat/keymap/` for keymap structure
