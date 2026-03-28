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

use std::io;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use error_stack::{Report, ResultExt};
use ratatui::{backend::CrosstermBackend, Terminal};
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct TerminalSuspendError;

/// RAII guard for terminal suspend/resume.
///
/// Suspends the TUI when created (exits raw mode, leaves alternate screen)
/// and automatically restores it when dropped. Used to temporarily return
/// to the normal terminal for external editor sessions.
pub struct TerminalGuard<'a> {
    terminal: &'a mut Terminal<CrosstermBackend<io::Stdout>>,
}

impl<'a> TerminalGuard<'a> {
    /// # Errors
    /// Returns an error if the terminal cannot be suspended.
    pub fn new(
        terminal: &'a mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<Self, Report<TerminalSuspendError>> {
        disable_raw_mode().change_context(TerminalSuspendError)?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .change_context(TerminalSuspendError)?;
        terminal
            .show_cursor()
            .change_context(TerminalSuspendError)?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard<'_> {
    fn drop(&mut self) {
        let _ = enable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        );
        let _ = self.terminal.hide_cursor();
        let _ = self.terminal.clear();
    }
}

impl TerminalGuard<'_> {
    pub fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        self.terminal
    }
}

/// Suspends the TUI, runs the closure, then resumes the TUI.
/// Automatically handles cleanup on drop, even if the closure panics.
///
/// # Errors
/// Returns an error if the terminal cannot be suspended or resumed.
pub fn suspend_and_run<F, T, E>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    f: F,
) -> Result<Result<T, E>, Report<TerminalSuspendError>>
where
    F: FnOnce() -> Result<T, E>,
{
    let _guard = TerminalGuard::new(terminal)?;
    Ok(f())
}
