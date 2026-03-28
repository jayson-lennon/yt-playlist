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

mod group_builder;
mod key;
mod map;
mod node;

pub use group_builder::GroupBuilder;
pub use key::{parse_key_sequence, Key};
pub use map::{FinalizeError, Keymap, MissingDescription};
pub use node::{KeyCategory, KeyChild, KeyContext, KeyNode, LeafBinding};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::{Pane, TuiAction};
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn key_display_shows_char() {
        // Given a character key
        let key = Key::Char('a');

        // When displaying the key
        let display = key.display();

        // Then it shows the character
        assert_eq!(display, "a");
    }

    #[test]
    fn key_display_shows_space() {
        // Given a space key
        let key = Key::Char(' ');

        // When displaying the key
        let display = key.display();

        // Then it shows "Space"
        assert_eq!(display, "Space");
    }

    #[rstest::rstest]
    #[case(Key::Tab, "Tab")]
    #[case(Key::Enter, "Enter")]
    #[case(Key::Backspace, "Bksp")]
    #[case(Key::Esc, "Esc")]
    #[case(Key::Up, "Up")]
    #[case(Key::Down, "Down")]
    #[case(Key::Left, "Left")]
    #[case(Key::Right, "Right")]
    #[case(Key::Home, "Home")]
    #[case(Key::End, "End")]
    #[case(Key::PageUp, "PgUp")]
    #[case(Key::PageDown, "PgDn")]
    fn key_display_special_keys(#[case] key: Key, #[case] expected: &str) {
        // Given a special key
        // When displaying the key
        // Then it shows the expected string
        assert_eq!(key.display(), expected);
    }

    #[test]
    fn parse_simple_chars() {
        // Given a string of simple characters
        let input = "abc";

        // When parsing the key sequence
        let keys = parse_key_sequence(input);

        // Then each character becomes a key
        assert_eq!(keys, vec![Key::Char('a'), Key::Char('b'), Key::Char('c')]);
    }

    #[test]
    fn parse_special_keys() {
        // Given a string with special key tags
        let input = "<Tab><Enter>";

        // When parsing the key sequence
        let keys = parse_key_sequence(input);

        // Then special keys are recognized
        assert_eq!(keys, vec![Key::Tab, Key::Enter]);
    }

    #[test]
    fn parse_mixed() {
        // Given a string with mixed characters and special keys
        let input = "g<Space>m";

        // When parsing the key sequence
        let keys = parse_key_sequence(input);

        // Then both are parsed correctly
        assert_eq!(keys, vec![Key::Char('g'), Key::Char(' '), Key::Char('m')]);
    }

    #[rstest::rstest]
    #[case(Pane::Playlist)]
    #[case(Pane::Library)]
    fn get_action_returns_global_action_in_any_pane(#[case] pane: Pane) {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting a global action
        let action = keymap.get_action(KeyCode::Char('q'), KeyModifiers::empty(), pane);

        // Then the action is returned
        assert_eq!(action, Some(TuiAction::Quit));
    }

    #[test]
    fn get_action_respects_playlist_context() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting a playlist-only action in playlist pane
        let action = keymap.get_action(KeyCode::Char('J'), KeyModifiers::empty(), Pane::Playlist);

        // Then the action is returned
        assert_eq!(action, Some(TuiAction::ReorderDown));
    }

    #[test]
    fn get_action_blocks_playlist_context_in_library() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting a playlist-only action in library pane
        let action = keymap.get_action(KeyCode::Char('J'), KeyModifiers::empty(), Pane::Library);

        // Then no action is returned
        assert!(action.is_none());
    }

    #[test]
    fn get_action_respects_library_context() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting a library-only action in library pane
        let action = keymap.get_action(KeyCode::Char('H'), KeyModifiers::empty(), Pane::Library);

        // Then the action is returned
        assert_eq!(action, Some(TuiAction::MoveToPlaylist));
    }

    #[test]
    fn get_action_blocks_library_context_in_playlist() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting a library-only action in playlist pane
        let action = keymap.get_action(KeyCode::Char('H'), KeyModifiers::empty(), Pane::Playlist);

        // Then no action is returned
        assert!(action.is_none());
    }

    #[rstest::rstest]
    #[case(Pane::Playlist)]
    #[case(Pane::Library)]
    fn get_action_returns_launch_file_in_any_pane(#[case] pane: Pane) {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting the launch file action
        let action = keymap.get_action(KeyCode::Char('o'), KeyModifiers::empty(), pane);

        // Then the action is returned
        assert_eq!(action, Some(TuiAction::LaunchFile));
    }

    #[test]
    fn get_action_returns_none_for_unbound_key() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting an unbound key
        let action = keymap.get_action(KeyCode::Char('z'), KeyModifiers::empty(), Pane::Playlist);

        // Then no action is returned
        assert!(action.is_none());
    }

    #[test]
    fn get_bindings_for_pane_includes_global_bindings() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting bindings for playlist pane
        let bindings = keymap.get_bindings_for_pane(Pane::Playlist);

        // Then global bindings are included
        assert!(bindings.iter().any(|b| b.action == TuiAction::Quit));
    }

    #[test]
    fn get_bindings_for_playlist_pane_includes_playlist_bindings() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting bindings for playlist pane
        let bindings = keymap.get_bindings_for_pane(Pane::Playlist);

        // Then playlist bindings are included
        assert!(bindings.iter().any(|b| b.action == TuiAction::ReorderUp));
    }

    #[test]
    fn get_bindings_for_library_pane_excludes_playlist_bindings() {
        // Given the default keymap
        let keymap = Keymap::new();

        let bindings = keymap.get_bindings_for_pane(Pane::Library);

        assert!(!bindings.iter().any(|b| b.action == TuiAction::ReorderUp));
    }

    #[test]
    fn get_bindings_for_library_pane_includes_library_bindings() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting bindings for library pane
        let bindings = keymap.get_bindings_for_pane(Pane::Library);

        // Then library bindings are included
        assert!(bindings
            .iter()
            .any(|b| b.action == TuiAction::MoveToPlaylist));
    }

    #[test]
    fn default_creates_keymap() {
        // When creating a default keymap
        let keymap = Keymap::default();

        // Then it has bindings
        let bindings = keymap.get_bindings_for_pane(Pane::Playlist);
        assert!(!bindings.is_empty());
    }

    #[test]
    fn bind_creates_leaf_node_at_path() {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When binding a single key
        keymap.bind(
            "x",
            TuiAction::Quit,
            "quit",
            KeyCategory::General,
            KeyContext::Global,
        );

        // Then a node exists at that path
        let node = keymap.get_node_at_path(&[Key::Char('x')]);
        assert!(node.is_some());
    }

    #[test]
    fn bind_leaf_has_correct_action() {
        // Given a keymap with a binding
        let mut keymap = Keymap::empty();
        keymap.bind(
            "x",
            TuiAction::Quit,
            "quit",
            KeyCategory::General,
            KeyContext::Global,
        );

        // When getting the node
        let node = keymap.get_node_at_path(&[Key::Char('x')]).unwrap();

        // Then it has the correct action
        assert!(matches!(
            node,
            KeyNode::Leaf {
                action: TuiAction::Quit,
                ..
            }
        ));
    }

    #[test]
    fn bind_leaf_has_correct_description() {
        // Given a keymap with a binding
        let mut keymap = Keymap::empty();
        keymap.bind(
            "x",
            TuiAction::Quit,
            "quit",
            KeyCategory::General,
            KeyContext::Global,
        );

        // When getting the node
        let node = keymap.get_node_at_path(&[Key::Char('x')]).unwrap();

        // Then it has the correct description
        assert_eq!(node.description(), "quit");
    }

    #[test]
    fn bind_leaf_has_correct_category() {
        // Given a keymap with a binding
        let mut keymap = Keymap::empty();
        keymap.bind(
            "x",
            TuiAction::Quit,
            "quit",
            KeyCategory::General,
            KeyContext::Global,
        );

        // When getting the node
        let node = keymap.get_node_at_path(&[Key::Char('x')]).unwrap();

        // Then it has the correct category
        assert_eq!(node.category(), Some(KeyCategory::General));
    }

    #[rstest::rstest]
    #[case(KeyContext::Global)]
    #[case(KeyContext::Playlist)]
    #[case(KeyContext::Library)]
    fn bind_leaf_has_correct_context(#[case] context: KeyContext) {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When binding with a specific context
        keymap.bind("x", TuiAction::Quit, "quit", KeyCategory::General, context);

        // Then the leaf has that context
        let node = keymap.get_node_at_path(&[Key::Char('x')]).unwrap();
        assert!(matches!(
            node,
            KeyNode::Leaf {
                context: c,
                ..
            } if *c == context
        ));
    }

    #[test]
    fn bind_creates_branch_for_multi_key_sequence() {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When binding a multi-key sequence
        keymap.bind(
            "gm",
            TuiAction::LaunchMpv,
            "launch mpv",
            KeyCategory::General,
            KeyContext::Global,
        );

        // Then the first key is a prefix key
        assert!(keymap.is_prefix_key(Key::Char('g')));

        // And a branch node exists at the first key
        let node = keymap.get_node_at_path(&[Key::Char('g')]).unwrap();
        assert!(node.is_branch());
    }

    #[test]
    fn finalize_fails_with_placeholder_description() {
        // Given a keymap with an undescribed branch
        let mut keymap = Keymap::empty();
        keymap.bind(
            "gm",
            TuiAction::LaunchMpv,
            "launch mpv",
            KeyCategory::General,
            KeyContext::Global,
        );

        // When finalizing
        let result = keymap.finalize();

        // Then it fails with missing description
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.missing_descriptions.len(), 1);
        assert_eq!(err.missing_descriptions[0].path, vec![Key::Char('g')]);
    }

    #[test]
    fn describe_sets_branch_description() {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When describing a prefix with bindings
        keymap.describe("g", "general", |g| {
            g.bind(
                "m",
                TuiAction::LaunchMpv,
                "launch mpv",
                KeyCategory::General,
                KeyContext::Global,
            );
        });

        // Then the branch has the description
        let node = keymap.get_node_at_path(&[Key::Char('g')]).unwrap();
        assert_eq!(node.description(), "general");
    }

    #[test]
    fn finalize_succeeds_when_branch_is_described() {
        // Given a keymap with a described branch
        let mut keymap = Keymap::empty();
        keymap.describe("g", "general", |g| {
            g.bind(
                "m",
                TuiAction::LaunchMpv,
                "launch mpv",
                KeyCategory::General,
                KeyContext::Global,
            );
        });

        // When finalizing
        let result = keymap.finalize();

        // Then it succeeds
        assert!(result.is_ok());
    }

    #[test]
    fn describe_creates_branch_with_description() {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When describing a prefix with multiple bindings
        keymap
            .describe("g", "general", |g| {
                g.bind(
                    "m",
                    TuiAction::LaunchMpv,
                    "launch mpv",
                    KeyCategory::General,
                    KeyContext::Global,
                );
            })
            .describe(" ", "leader", |leader| {
                leader.describe("s", "search", |s| {
                    s.bind(
                        "f",
                        TuiAction::FuzzyNotes,
                        "fuzzy notes",
                        KeyCategory::General,
                        KeyContext::Global,
                    );
                });
            });

        // Then the branch has the description
        let node = keymap.get_node_at_path(&[Key::Char('g')]).unwrap();
        assert_eq!(node.description(), "general");
    }

    #[rstest::rstest]
    #[case(&[Key::Char('g'), Key::Char('m')], TuiAction::LaunchMpv)]
    #[case(&[Key::Char(' '), Key::Char('s'), Key::Char('f')], TuiAction::FuzzyNotes)]
    fn describe_creates_leaf_children(#[case] path: &[Key], #[case] expected_action: TuiAction) {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When describing a prefix with multiple bindings
        keymap
            .describe("g", "general", |g| {
                g.bind(
                    "m",
                    TuiAction::LaunchMpv,
                    "launch mpv",
                    KeyCategory::General,
                    KeyContext::Global,
                );
            })
            .describe(" ", "leader", |leader| {
                leader.describe("s", "search", |s| {
                    s.bind(
                        "f",
                        TuiAction::FuzzyNotes,
                        "fuzzy notes",
                        KeyCategory::General,
                        KeyContext::Global,
                    );
                });
            });

        // Then each path has the correct leaf action
        let node = keymap.get_node_at_path(path).unwrap();
        assert!(matches!(
            node,
            KeyNode::Leaf {
                action,
                ..
            } if *action == expected_action
        ));
    }

    #[rstest::rstest]
    #[case(Key::Char('g'), "general")]
    #[case(Key::Char('a'), "add")]
    fn describe_chains_multiple_prefixes(#[case] key: Key, #[case] expected_description: &str) {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When chaining multiple describe calls
        keymap
            .describe("g", "general", |g| {
                g.bind(
                    "m",
                    TuiAction::LaunchMpv,
                    "launch mpv",
                    KeyCategory::General,
                    KeyContext::Global,
                );
            })
            .describe("a", "add", |a| {
                a.bind(
                    "u",
                    TuiAction::AddUrl,
                    "add url",
                    KeyCategory::General,
                    KeyContext::Global,
                );
            });

        // Then each prefix has its description
        let node = keymap.get_node_at_path(&[key]).unwrap();
        assert_eq!(node.description(), expected_description);
    }

    #[test]
    fn finalize_detects_multiple_missing_descriptions() {
        // Given a keymap with multiple undescribed branches
        let mut keymap = Keymap::empty();
        keymap.bind(
            "gm",
            TuiAction::LaunchMpv,
            "launch mpv",
            KeyCategory::General,
            KeyContext::Global,
        );
        keymap.bind(
            "au",
            TuiAction::AddUrl,
            "add url",
            KeyCategory::General,
            KeyContext::Global,
        );

        // When finalizing
        let result = keymap.finalize();

        // Then it fails with all missing descriptions
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.missing_descriptions.len(), 2);
    }

    #[test]
    fn bind_adds_multiple_children_to_branch() {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When binding multiple keys under the same prefix
        keymap.describe("g", "general", |g| {
            g.bind(
                "m",
                TuiAction::LaunchMpv,
                "launch mpv",
                KeyCategory::General,
                KeyContext::Global,
            )
            .bind(
                "d",
                TuiAction::Delete,
                "delete",
                KeyCategory::General,
                KeyContext::Global,
            );
        });

        // Then the branch has multiple children
        let children = keymap.get_children_at_path(&[Key::Char('g')]).unwrap();
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn bind_supports_nested_describes() {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When nesting describes
        keymap.describe("g", "general", |g| {
            g.describe("m", "mpv", |m| {
                m.bind(
                    "p",
                    TuiAction::LaunchMpv,
                    "mpv play",
                    KeyCategory::General,
                    KeyContext::Global,
                );
            });
        });

        // Then the first key is a prefix
        assert!(keymap.is_prefix_key(Key::Char('g')));
    }

    #[rstest::rstest]
    #[case(&[Key::Char('g')], "general")]
    #[case(&[Key::Char('g'), Key::Char('m')], "mpv")]
    fn bind_nested_describe_has_descriptions(#[case] path: &[Key], #[case] expected: &str) {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When nesting describes
        keymap.describe("g", "general", |g| {
            g.describe("m", "mpv", |m| {
                m.bind(
                    "p",
                    TuiAction::LaunchMpv,
                    "mpv play",
                    KeyCategory::General,
                    KeyContext::Global,
                );
            });
        });

        // Then each level has its description
        let node = keymap.get_node_at_path(path).unwrap();
        assert!(node.is_branch());
        assert_eq!(node.description(), expected);
    }

    #[test]
    fn bind_nested_creates_leaf_at_full_path() {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When nesting describes with a leaf binding
        keymap.describe("g", "general", |g| {
            g.describe("m", "mpv", |m| {
                m.bind(
                    "p",
                    TuiAction::LaunchMpv,
                    "mpv play",
                    KeyCategory::General,
                    KeyContext::Global,
                );
            });
        });

        // Then the leaf exists at the full path
        let node = keymap
            .get_node_at_path(&[Key::Char('g'), Key::Char('m'), Key::Char('p')])
            .unwrap();
        assert!(matches!(
            node,
            KeyNode::Leaf {
                action: TuiAction::LaunchMpv,
                ..
            }
        ));
    }

    #[test]
    fn get_children_at_path_returns_none_for_leaf() {
        // Given a keymap with a leaf binding
        let mut keymap = Keymap::empty();
        keymap.bind(
            "x",
            TuiAction::Quit,
            "quit",
            KeyCategory::General,
            KeyContext::Global,
        );

        // When getting children at a leaf path
        let children = keymap.get_children_at_path(&[Key::Char('x')]);

        // Then no children are returned
        assert!(children.is_none());
    }

    #[test]
    fn get_node_at_path_returns_none_for_empty() {
        // Given an empty keymap
        let keymap = Keymap::empty();

        // When getting node with empty path
        let node = keymap.get_node_at_path(&[]);

        // Then no node is returned
        assert!(node.is_none());
    }

    #[test]
    fn get_node_at_path_returns_none_for_unknown_key() {
        // Given an empty keymap
        let keymap = Keymap::empty();

        // When getting node for unbound key
        let node = keymap.get_node_at_path(&[Key::Char('z')]);

        // Then no node is returned
        assert!(node.is_none());
    }

    #[rstest::rstest]
    #[case(Key::Char('g'), true, "prefix key")]
    #[case(Key::Char('m'), false, "leaf key")]
    #[case(Key::Char('x'), false, "unbound key")]
    fn is_prefix_key_returns_correct_value(
        #[case] key: Key,
        #[case] expected: bool,
        #[case] description: &str,
    ) {
        // Given a keymap with a prefix key 'g'
        let mut keymap = Keymap::empty();
        keymap.describe("g", "general", |g| {
            g.bind(
                "m",
                TuiAction::LaunchMpv,
                "launch mpv",
                KeyCategory::General,
                KeyContext::Global,
            );
        });

        // When checking if key is a prefix
        // Then it returns the expected value
        assert_eq!(
            keymap.is_prefix_key(key),
            expected,
            "failed for {description}"
        );
    }

    #[rstest::rstest]
    #[case(Key::Char('g'), "general prefix")]
    #[case(Key::Char('a'), "add prefix")]
    fn default_keymap_has_prefix_keys(#[case] key: Key, #[case] description: &str) {
        // Given the default keymap
        let keymap = Keymap::new();

        // When checking for prefix keys
        // Then they are recognized as prefixes
        assert!(keymap.is_prefix_key(key), "failed for {description}");
    }

    #[rstest::rstest]
    #[case(&[Key::Char('g'), Key::Char('m')], TuiAction::LaunchMpv)]
    #[case(&[Key::Char('a'), Key::Char('u')], TuiAction::AddUrl)]
    fn default_keymap_has_sequence_leaf_bindings(
        #[case] path: &[Key],
        #[case] expected_action: TuiAction,
    ) {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting a node at a sequence path
        let node = keymap.get_node_at_path(path).unwrap();

        // Then it has the correct action
        assert!(matches!(
            node,
            KeyNode::Leaf {
                action,
                ..
            } if *action == expected_action
        ));
    }

    #[test]
    fn default_keymap_has_all_descriptions() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When finalizing
        let result = keymap.finalize();

        // Then it succeeds
        assert!(result.is_ok());
    }

    #[test]
    fn bind_with_special_key() {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When binding with a special key
        keymap.bind(
            "<Tab>",
            TuiAction::SwitchPane,
            "switch pane",
            KeyCategory::PaneSwitch,
            KeyContext::Global,
        );

        // Then the node exists with the correct action
        let node = keymap.get_node_at_path(&[Key::Tab]);
        assert!(matches!(
            node,
            Some(KeyNode::Leaf {
                action: TuiAction::SwitchPane,
                ..
            })
        ));
    }

    #[test]
    fn missing_description_display() {
        // Given a missing description with a path
        let missing = MissingDescription {
            path: vec![Key::Char('g'), Key::Char('m')],
        };

        // When displaying the error
        let display = missing.to_string();

        // Then it shows the key sequence
        assert_eq!(display, "Key sequence 'gm' is missing a description");
    }

    #[test]
    fn finalize_error_display() {
        // Given a finalize error with multiple missing descriptions
        let err = FinalizeError {
            missing_descriptions: vec![
                MissingDescription {
                    path: vec![Key::Char('g')],
                },
                MissingDescription {
                    path: vec![Key::Char('a')],
                },
            ],
        };

        // When displaying the error
        let display = err.to_string();

        // Then it contains all missing descriptions
        assert!(display.contains("Key sequence 'g' is missing a description"));
        assert!(display.contains("Key sequence 'a' is missing a description"));
    }
}
