use ratatui::{layout::Rect, Frame};

use crate::feat::keymap::Keymap;
use crate::services::Services;

use super::state::TuiState;

pub struct RenderContext<'a, 'frame> {
    pub frame: &'a mut Frame<'frame>,
    pub area: Rect,
    pub keymap: &'a Keymap,
    pub services: &'a Services,
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
        let keymap = KEYMAP.get_or_init(Keymap::new);
        let tui_state = TUI_STATE.get_or_init(TuiState::new);
        let services = get_services();

        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 80, 24);

        terminal
            .draw(|frame| {
                let ctx = RenderContext {
                    frame,
                    area,
                    keymap,
                    services,
                    tui_state,
                };
                assert_eq!(ctx.area, area);
            })
            .unwrap();
    }

    #[test]
    fn dummy_component_implements_render() {
        fn assert_render<T: Render>() {}
        assert_render::<DummyComponent>();
    }
}
