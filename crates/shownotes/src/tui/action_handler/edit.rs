use super::TuiActionCtx;
use super::TuiActionError;
use crate::command::Command;
use crate::tui::ShowNoteKind;
use crate::tui::TuiActionResponse;
use error_stack::{Report, ResultExt};

pub fn handle_notes(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    if let Some(item) = ctx.tui_state.get_selected_item() {
        ctx.fork.notes_path = Some(item.path.clone());
    }
    TuiActionResponse::Continue
}

pub fn handle_fuzzy_notes(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    ctx.fork.fuzzy_notes = true;
    TuiActionResponse::Continue
}

pub fn handle_edit_sources(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    if let Some(item) = ctx.tui_state.get_selected_item() {
        ctx.fork.sources_path = Some(item.path.clone());
    }
    TuiActionResponse::Continue
}

pub fn handle_rename(ctx: &mut TuiActionCtx<'_>) -> TuiActionResponse {
    ctx.tui_state.start_rename();
    TuiActionResponse::Continue
}

pub fn handle_generate_show_notes(
    ctx: &mut TuiActionCtx<'_>,
    kind: ShowNoteKind,
) -> TuiActionResponse {
    ctx.fork.generate_notes = Some(kind.as_str().to_string());
    TuiActionResponse::Continue
}

pub fn handle_rename_submit(
    ctx: &mut TuiActionCtx<'_>,
    alias: String,
) -> Result<TuiActionResponse, Report<TuiActionError>> {
    if let Some(item) = ctx.tui_state.get_selected_item_mut() {
        let old_alias = item.alias.clone();
        item.alias = Some(alias.clone());

        if old_alias.as_deref() != Some(&alias) {
            let path = item.path.clone();

            if let Some(file_path) = path.as_file() {
                ctx.execute(Command::AliasSet {
                    path: file_path.clone(),
                    workspace: ctx.ctx.library_path.clone(),
                    alias,
                })
                .change_context(TuiActionError)?;
            }
        }
    }
    Ok(TuiActionResponse::Continue)
}
