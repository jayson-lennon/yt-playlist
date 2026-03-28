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

use ratatui::{layout::Rect, Frame};

use crate::feat::keymap::Keymap;
use crate::services::Services;

use super::state::TuiState;

/// Context for rendering operations.
///
/// Provides access to the frame for drawing, the target area, and shared state
/// needed by components during rendering.
pub struct RenderContext<'a, 'frame> {
    /// The ratatui frame to render into.
    pub frame: &'a mut Frame<'frame>,
    /// The target area for rendering.
    pub area: Rect,
    /// Keymap for key binding lookups.
    pub keymap: &'a Keymap,
    /// Services for async operations.
    pub services: &'a Services,
    /// Current TUI state.
    pub tui_state: &'a TuiState,
}

impl<'a, 'frame> RenderContext<'a, 'frame> {
    pub fn new(
        frame: &'a mut Frame<'frame>,
        area: Rect,
        keymap: &'a Keymap,
        services: &'a Services,
        tui_state: &'a TuiState,
    ) -> Self {
        Self {
            frame,
            area,
            keymap,
            services,
            tui_state,
        }
    }
}

/// Builder for rendering a component to a specific area.
///
/// Modifies the context's area before rendering, allowing reuse of a single
/// context across multiple components with different areas.
pub struct AreaRender {
    area: Rect,
}

impl AreaRender {
    /// Create a new AreaRender targeting the specified area.
    pub fn to(area: Rect) -> Self {
        Self { area }
    }

    /// Render a component to this builder's area.
    ///
    /// Sets `ctx.area` to the builder's area, then calls `Render::render`.
    pub fn render<C: Render>(self, ctx: &mut RenderContext<'_, '_>, component: &C) {
        ctx.area = self.area;
        Render::render(component, ctx);
    }

    /// Try to render a component to this builder's area.
    ///
    /// Sets `ctx.area` to the builder's area, then calls `Render::try_render`.
    pub fn try_render<C: Render>(self, ctx: &mut RenderContext<'_, '_>, component: &C) {
        ctx.area = self.area;
        Render::try_render(component, ctx);
    }
}

pub trait Render {
    fn should_render(&self, ctx: &RenderContext<'_, '_>) -> bool;
    fn render(&self, ctx: &mut RenderContext<'_, '_>);
    fn try_render(&self, ctx: &mut RenderContext<'_, '_>) {
        if self.should_render(ctx) {
            self.render(ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use super::*;

    struct DummyComponent;

    impl Render for DummyComponent {
        fn should_render(&self, _ctx: &RenderContext<'_, '_>) -> bool {
            true
        }
        fn render(&self, _ctx: &mut RenderContext<'_, '_>) {}
    }

    static KEYMAP: OnceLock<Keymap> = OnceLock::new();
    static TUI_STATE: OnceLock<TuiState> = OnceLock::new();
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    static SERVICES: OnceLock<Services> = OnceLock::new();

    fn get_services() -> &'static Services {
        SERVICES.get_or_init(|| {
            let rt = RT.get_or_init(|| tokio::runtime::Runtime::new().unwrap());
            rt.block_on(Services::new(":memory:", rt.handle().clone()))
                .unwrap()
        })
    }

    #[test]
    fn render_context_has_required_fields() {
        // Given a terminal and required dependencies.
        let keymap = KEYMAP.get_or_init(Keymap::new);
        let tui_state = TUI_STATE.get_or_init(TuiState::new);
        let services = get_services();

        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 80, 24);

        // When creating a RenderContext within a terminal draw.
        terminal
            .draw(|frame| {
                let ctx = RenderContext {
                    frame,
                    area,
                    keymap,
                    services,
                    tui_state,
                };
                // Then the context has the expected area.
                assert_eq!(ctx.area, area);
            })
            .unwrap();
    }

    #[test]
    fn dummy_component_implements_render() {
        // Given a type assertion function for Render trait.
        fn assert_render<T: Render>() {}

        // When checking if DummyComponent implements Render.
        // Then the assertion compiles without error.
        assert_render::<DummyComponent>();
    }
}
