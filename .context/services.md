# Services Architecture

## Overview

Services are the injectable building blocks that encapsulate external dependencies (databases, file systems, external processes). Each service wraps a trait object backend, enabling dependency injection and testability through swappable implementations. Services are aggregated in a `Services` container and accessed via `SystemCtx` in command handlers.

## Core Types

### Backend Trait (in `feat/<domain>/mod.rs`)

- **Purpose**: Defines the contract for a domain's operations, implemented by real and fake backends
- **Key requirements**: `Send + Sync` bounds, `#[async_trait]` for async methods, `name()` method for debugging
- **Usage**: Implemented by real backends (e.g., `SqliteNoteDb`) and test fakes (e.g., `FakeMpvBackend`)

### Service Wrapper (in `feat/<domain>/mod.rs`)

- **Purpose**: Wraps `Arc<dyn Trait>` for ergonomic access and Clone semantics
- **Key fields**: `backend: Arc<dyn SomeTrait>` with `#[debug("<TraitName>")]` annotation
- **Usage**: Exposed via `Services` struct, delegates all methods to backend

### `Services` (`crates/shownotes/src/services.rs`)

- **Purpose**: Container holding all service instances for dependency injection
- **Key fields**: One field per service (`db`, `storage`, `media`, `mpv`, etc.) plus `rt: tokio::runtime::Handle`
- **Usage**: Created at app startup, cloned cheaply throughout the app, accessed via `ctx.services.*`

### Error Types (in `feat/<domain>/mod.rs`)

- **Purpose**: Domain-specific error types using `wherror::Error` derive
- **Usage**: Wrapped in `error_stack::Report<ErrorType>` for rich error context

## Data Flow

```
Services::new(db_path, rt)
        ↓
    Create shared resources (SqliteNoteDb, pool)
        ↓
    Create backend implementations (Arc<dyn Trait>)
        ↓
    Wrap in Service structs
        ↓
    Return Services container
        ↓
    Clone and pass to SystemCtx
        ↓
    Access via ctx.services.<field>
        ↓
    Call service methods (delegates to backend)
```

## Architectural Patterns

1. **Trait-Based Backend Pattern**: Each service has a trait defining its interface, with multiple implementations (real + fake)
2. **Arc Wrapper Pattern**: Services wrap `Arc<dyn Trait>` for shared ownership and dynamic dispatch
3. **Dependency Injection**: Services are created centrally and injected where needed
4. **Clone Semantics**: Services implement Clone (cheap via Arc) for easy sharing
5. **Debug Customization**: `#[debug("<TraitName>")]` provides meaningful debug output

## Modification Guide

When creating a new service:

1. **Create the feature module**: `crates/shownotes/src/feat/<domain>/mod.rs`

2. **Define the error type**:

   ```rust
   use wherror::Error;

   #[derive(Debug, Error)]
   #[error("description of failure")]
   pub struct YourDomainError;
   ```

3. **Define the backend trait**:

   ```rust
   #[async_trait]  // if async methods needed
   pub trait YourDomain: Send + Sync {
       fn name(&self) -> &'static str;
       fn operation(&self, param: &str) -> Result<Output, Report<YourDomainError>>;
   }
   ```

4. **Create the service wrapper**:

   ```rust
   #[derive(Debug, Clone)]
   pub struct YourDomainService {
       #[debug("<YourDomain>")]
       backend: Arc<dyn YourDomain>,
   }

   impl YourDomainService {
       pub fn new(backend: Arc<dyn YourDomain>) -> Self {
           Self { backend }
       }
   }

   impl YourDomain for YourDomainService {
       fn name(&self) -> &'static str { self.backend.name() }
       fn operation(&self, param: &str) -> Result<Output, Report<YourDomainError>> {
           self.backend.operation(param)
       }
   }
   ```

5. **Implement real backend**: Create in `feat/<domain>/backends/` or inline in `mod.rs`

6. **Create test fake** in `test_utils/fakes.rs`:

   ```rust
   pub struct FakeYourDomain;
   impl YourDomain for FakeYourDomain {
       fn name(&self) -> &'static str { "fake" }
       fn operation(&self, _: &str) -> Result<Output, Report<YourDomainError>> { Ok(Default::default()) }
   }
   ```

7. **Add to Services struct** in `services.rs`:

   ```rust
   pub struct Services {
       // ... existing fields
       pub your_domain: YourDomainService,
   }
   ```

8. **Wire in Services::new()**:

   ```rust
   your_domain: YourDomainService::new(Arc::new(RealBackend::new())),
   ```

9. **Add to test services** in `test_utils/services.rs`:

   ```rust
   your_domain: YourDomainService::new(Arc::new(FakeYourDomain)),
   ```

10. **Export from feat/mod.rs**:

    ```rust
    pub use your_domain::{YourDomain, YourDomainError, YourDomainService, RealBackend};
    ```

11. **Export from lib.rs** if needed by external consumers

## Key Files Reference

| Purpose               | File Path                                          |
| --------------------- | -------------------------------------------------- |
| Services container    | `crates/shownotes/src/services.rs`                 |
| Feature modules       | `crates/shownotes/src/feat/*/mod.rs`               |
| Feature exports       | `crates/shownotes/src/feat/mod.rs`                 |
| Library exports       | `crates/shownotes/src/lib.rs`                      |
| Test fakes            | `crates/shownotes/src/test_utils/fakes.rs`         |
| Test services factory | `crates/shownotes/src/test_utils/services.rs`      |
| Example service       | `crates/shownotes/src/feat/external_editor/mod.rs` |

## Related Contexts

Search for potentially related context files in `.context/` based on the user's request. For example, if only adding a CLI flag that uses existing code, only CLI context may be needed. If adding an entirely new feature, multiple contexts (CLI + business rules + TUI) may be relevant.
