Feature: Which-Key Popup
  The which-key popup shows available keybindings and handles key sequences.
  When the popup is showing, pressing keys should dismiss it and process the key.

  Scenario: Pressing a prefix key from help popup dismisses and shows sub-menu
    Given the which-key popup is showing
    When I press the prefix key "g"
    Then the which-key is pending with key "g"

  Scenario: Pressing an action key from help popup dismisses and returns action
    Given the which-key popup is showing
    When I press the action key "q"
    Then the which-key popup is dismissed
    And the result contains the Quit action

  Scenario: Pressing Escape from help popup dismisses without action
    Given the which-key popup is showing
    When I press Escape
    Then the which-key popup is dismissed
    And the result contains no actions
