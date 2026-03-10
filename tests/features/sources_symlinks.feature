Feature: Sources command symlink handling
  The 'sources' CLI commands should follow symlinks to canonical paths
  and operate on the real underlying file.

  Scenario: 'sources list' follows symlink to real file
    Given a real file at "real.mp4"
    And a symlink to "real.mp4" at "link.mp4"
    And the file "real.mp4" has source "https://example.com/video"
    When I run "sources list link.mp4"
    Then the output contains "https://example.com/video"

  Scenario: 'sources add' stores to canonical path
    Given a real file at "real.mp4"
    And a symlink to "real.mp4" at "link.mp4"
    When I run "sources add link.mp4 https://example.com/source"
    Then the file "real.mp4" has source "https://example.com/source"

  Scenario: 'sources edit' applies to canonical file
    Given a real file at "real.mp4"
    And a symlink to "real.mp4" at "link.mp4"
    And the file "real.mp4" has source "https://old.example.com"
    When I edit sources for "link.mp4" with "https://new.example.com"
    Then the file "real.mp4" has source "https://new.example.com"
    And the file "link.mp4" shows source "https://new.example.com"

  Scenario: Adding via symlink, listing via real file
    Given a real file at "real.mp4"
    And a symlink to "real.mp4" at "link.mp4"
    When I run "sources add link.mp4 https://example.com/test"
    And I run "sources list real.mp4"
    Then the output contains "https://example.com/test"

  Scenario: Chained symlinks resolve to final target
    Given a real file at "real.mp4"
    And a symlink to "real.mp4" at "link1.mp4"
    And a symlink to "link1.mp4" at "link2.mp4"
    When I run "sources add link2.mp4 https://example.com/chained"
    Then the file "real.mp4" has source "https://example.com/chained"
