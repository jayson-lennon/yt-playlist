Feature: Notes CLI operations
  Users can add, search, and manage notes for files.

  Scenario: Add note to a file
    Given a file "video.mp4"
    When I add note "Great tutorial about Rust" to "video.mp4"
    Then the file "video.mp4" has note "Great tutorial about Rust"

  Scenario: Add note appends to existing notes
    Given a file "video.mp4" with note "First note"
    When I add note "Second note" to "video.mp4"
    Then the file "video.mp4" has note "First note"
    And the file "video.mp4" has note "Second note"

  Scenario: Search notes finds matching files
    Given a file "rust-tutorial.mp4" with note "Learn Rust programming"
    And a file "python-tutorial.mp4" with note "Learn Python programming"
    When I search notes for "Rust"
    Then the output contains "rust-tutorial.mp4"
    And the output does not contain "python-tutorial.mp4"

  Scenario: Search notes with symlink creation
    Given a file "video.mp4" with note "Important content"
    When I search notes for "Important" with symlinks
    Then a symlink to "video.mp4" exists in current directory

  Scenario: Search with no matches returns empty
    Given a file "video.mp4" with note "Some content"
    When I search notes for "nonexistent"
    Then the output is empty
