#![allow(clippy::missing_panics_doc)]
use cucumber::{World, given, then, when};

use acceptance::ShownotesWorld;
use marked_path::CanonicalPath;
use shownotes::command::{Command, execute};

#[derive(Debug, World)]
#[world(init = Self::new_world)]
pub struct AliasDeletionWorld {
    inner: ShownotesWorld,
}

impl AliasDeletionWorld {
    async fn new_world() -> Self {
        Self {
            inner: ShownotesWorld::new().await,
        }
    }
}

#[given(expr = r#"a real file at {string}"#)]
fn given_real_file(world: &mut AliasDeletionWorld, filename: String) {
    world.inner.create_file(&filename);
}

#[given(expr = r#"the file {string} has alias {string}"#)]
async fn given_file_has_alias(world: &mut AliasDeletionWorld, path: String, alias: String) {
    let full_path = world.inner.resolve_path(&path);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");
    let workspace = CanonicalPath::from_path(world.inner.temp_dir.path())
        .expect("failed to canonicalize workspace");

    execute(
        &world.inner.ctx,
        Command::AliasSet {
            path: canonical,
            workspace,
            alias,
        },
    )
    .await
    .expect("failed to set alias");
}

#[when(expr = r#"I remove the alias from {string}"#)]
async fn when_remove_alias(world: &mut AliasDeletionWorld, path: String) {
    let full_path = world.inner.resolve_path(&path);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");
    let workspace = CanonicalPath::from_path(world.inner.temp_dir.path())
        .expect("failed to canonicalize workspace");

    execute(
        &world.inner.ctx,
        Command::AliasRemove {
            path: canonical,
            workspace,
        },
    )
    .await
    .expect("failed to remove alias");
}

#[then(expr = r#"the file {string} has no alias"#)]
async fn then_file_has_no_alias(world: &mut AliasDeletionWorld, path: String) {
    let full_path = world.inner.resolve_path(&path);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");
    let workspace = CanonicalPath::from_path(world.inner.temp_dir.path())
        .expect("failed to canonicalize workspace");

    let alias = world
        .inner
        .ctx
        .services
        .storage
        .resolve_alias(&canonical, &workspace)
        .await
        .expect("failed to resolve alias");

    assert!(
        alias.is_none(),
        "expected file '{path}' to have no alias, but found: '{alias:?}'"
    );
}

#[tokio::main]
async fn main() {
    AliasDeletionWorld::run("tests/features/alias_deletion.feature").await;
}
