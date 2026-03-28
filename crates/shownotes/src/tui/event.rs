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

use crate::tui::TuiAction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStatus {
    Consumed,
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandleKeyResult {
    pub status: KeyStatus,
    pub actions: Vec<TuiAction>,
}

impl HandleKeyResult {
    pub fn ignored() -> Self {
        Self {
            status: KeyStatus::Ignored,
            actions: Vec::new(),
        }
    }

    pub fn consumed() -> Self {
        Self {
            status: KeyStatus::Consumed,
            actions: Vec::new(),
        }
    }

    pub fn with_action(action: TuiAction) -> Self {
        Self {
            status: KeyStatus::Consumed,
            actions: vec![action],
        }
    }

    pub fn is_consumed(&self) -> bool {
        self.status == KeyStatus::Consumed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignored_returns_ignored_status() {
        // Given no specific setup needed.

        // When creating an ignored result.
        let result = HandleKeyResult::ignored();

        // Then it is not consumed and has no actions.
        assert!(!result.is_consumed());
        assert!(result.actions.is_empty());
    }

    #[test]
    fn consumed_returns_consumed_status() {
        // Given no specific setup needed.

        // When creating a consumed result.
        let result = HandleKeyResult::consumed();

        // Then it is consumed and has no actions.
        assert!(result.is_consumed());
        assert!(result.actions.is_empty());
    }

    #[test]
    fn with_action_returns_consumed_status_with_action() {
        // Given a TuiAction.
        let action = TuiAction::Quit;

        // When creating a result with that action.
        let result = HandleKeyResult::with_action(action.clone());

        // Then it is consumed and contains the action.
        assert!(result.is_consumed());
        assert_eq!(result.actions, vec![action]);
    }

    #[test]
    fn is_consumed_returns_true_for_consumed() {
        // Given a consumed result.
        let result = HandleKeyResult::consumed();

        // When checking if consumed.
        // Then it returns true.
        assert!(result.is_consumed());
    }

    #[test]
    fn is_consumed_returns_false_for_ignored() {
        // Given an ignored result.
        let result = HandleKeyResult::ignored();

        // When checking if consumed.
        // Then it returns false.
        assert!(!result.is_consumed());
    }

    #[test]
    fn key_status_equality_works() {
        // Given no specific setup needed.

        // When comparing KeyStatus values.
        // Then equality works as expected.
        assert_eq!(KeyStatus::Consumed, KeyStatus::Consumed);
        assert_eq!(KeyStatus::Ignored, KeyStatus::Ignored);
        assert_ne!(KeyStatus::Consumed, KeyStatus::Ignored);
    }
}
