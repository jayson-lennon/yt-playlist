use super::TuiActionCtx;
use crate::tui::ShowNoteKind;
use crate::tui::TuiActionResponse;

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
