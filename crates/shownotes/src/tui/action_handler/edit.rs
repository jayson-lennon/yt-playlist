// Copyright (C) 2026 Jayson Lennon
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// 
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
