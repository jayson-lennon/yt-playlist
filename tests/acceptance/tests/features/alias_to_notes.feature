Feature: Alias to Notes Migration
  When an alias is added to a file, it should automatically be saved as a note.
  The system should prevent duplicate notes by checking for exact matches
  and substring matches.

  Scenario: Adding an alias saves it as a note
    Given a real file at "video.mp4"
    When I add alias "My Video Title" to "video.mp4"
    Then the file "video.mp4" has note "My Video Title"

  Scenario: Blank alias is not saved as a note
    Given a real file at "video.mp4"
    When I add alias "" to "video.mp4"
    Then the file "video.mp4" has no notes

  Scenario: Exact duplicate alias is not added
    Given a real file at "video.mp4"
    And the file "video.mp4" has note "existing note"
    When I add alias "existing note" to "video.mp4"
    Then the file "video.mp4" has note "existing note"
    And the file "video.mp4" has exactly 1 note line

  Scenario: Substring match prevents adding alias
    Given a real file at "video.mp4"
    And the file "video.mp4" has note "foo bar baz"
    When I add alias "foo" to "video.mp4"
    Then the file "video.mp4" has note "foo bar baz"
    And the file "video.mp4" has exactly 1 note line

  Scenario: New alias is appended to existing notes
    Given a real file at "video.mp4"
    And the file "video.mp4" has note "first note"
    When I add alias "second note" to "video.mp4"
    Then the file "video.mp4" has note "first note"
    And the file "video.mp4" has note "second note"

  Scenario: Alias via symlink saves to real file
    Given a real file at "real.mp4"
    And a symlink to "real.mp4" at "link.mp4"
    When I add alias "My Alias" to "link.mp4"
    Then the file "real.mp4" has note "My Alias"

  Scenario: Chained symlinks resolve to real file
    Given a real file at "real.mp4"
    And a symlink to "real.mp4" at "link1.mp4"
    And a symlink to "link1.mp4" at "link2.mp4"
    When I add alias "Chained Alias" to "link2.mp4"
    Then the file "real.mp4" has note "Chained Alias"
