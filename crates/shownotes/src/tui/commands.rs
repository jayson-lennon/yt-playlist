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

use std::collections::HashSet;

use crate::command::{self, Command, CommandResult};
use crate::feat::sources::SourceDb;
use crate::system_ctx::SystemCtx;
use crate::tui::common::ItemPath;
use crate::tui::{Pane, TuiState};

pub fn load_playlist(ctx: &SystemCtx, tui_state: &mut TuiState) {
    match ctx
        .services
        .rt
        .block_on(command::execute(ctx, Command::PlaylistLoad))
    {
        Ok(CommandResult::PlaylistLoaded {
            playlist_items,
            virtual_library_items,
        }) => {
            tui_state.playlist_pane.items = playlist_items;
            refresh_library(ctx, tui_state);
            for item in virtual_library_items {
                tui_state.library_pane.items.push(item);
            }
            tui_state
                .library_pane
                .items
                .sort_by(|a, b| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()));
            update_sources_status(ctx, tui_state);
        }
        Err(e) => {
            tui_state.show_error(format!("Failed to load playlist: {e:?}"));
        }
        _ => unreachable!(),
    }
}

pub fn refresh_library(ctx: &SystemCtx, tui_state: &mut TuiState) {
    match ctx
        .services
        .rt
        .block_on(command::execute(ctx, Command::LibraryRefresh))
    {
        Ok(CommandResult::LibraryRefreshed { items }) => {
            tui_state.refresh_library(items);
        }
        Err(e) => {
            tui_state.show_error(format!("Failed to refresh library: {e:?}"));
        }
        _ => unreachable!(),
    }
}

pub fn set_initial_focus(tui_state: &mut TuiState) {
    let playlist_empty = tui_state.playlist_pane.items.is_empty();
    let library_empty = tui_state.library_pane.items.is_empty();
    if playlist_empty && !library_empty {
        tui_state.focused_pane = Pane::Library;
    } else if library_empty && !playlist_empty {
        tui_state.focused_pane = Pane::Playlist;
    }
}

pub fn update_sources_status(ctx: &SystemCtx, tui_state: &mut TuiState) {
    let paths: Vec<String> = tui_state
        .playlist_pane
        .items
        .iter()
        .chain(tui_state.library_pane.items.iter())
        .filter_map(|item| match &item.path {
            ItemPath::File(path) => Some(path.as_path().to_string_lossy().into_owned()),
            ItemPath::Url(_) => None,
        })
        .collect();

    let paths_with_sources: HashSet<ItemPath> = match ctx
        .services
        .rt
        .block_on(ctx.services.sources.get_sources_for_paths(&paths))
    {
        Ok(sources_map) => tui_state
            .playlist_pane
            .items
            .iter()
            .chain(tui_state.library_pane.items.iter())
            .filter(|item| matches!(item.path, ItemPath::File(_)))
            .filter(|item| {
                let path_str = item.path.to_string_lossy();
                sources_map
                    .get(path_str.as_ref())
                    .is_some_and(|sources| !sources.is_empty())
            })
            .map(|item| item.path.clone())
            .collect(),
        Err(e) => {
            tracing::error!("Failed to get sources for paths: {e:?}");
            HashSet::new()
        }
    };

    tui_state.update_sources_status(&paths_with_sources);
}
