Feature: Generate show notes from playlist
  Users can generate formatted show notes from their playlist with source URLs.

  Scenario: Generate markdown format with sources
    Given a file "video1.mp4" with source "https://youtube.com/watch?v=abc"
    And a file "video2.mp4" with source "https://youtube.com/watch?v=def"
    When I generate show notes in "markdown" format
    Then the output contains "video1"
    And the output contains "https://youtube.com/watch?v=abc"
    And the output contains "video2"
    And the output contains "https://youtube.com/watch?v=def"

  Scenario: Generate HTML format with sources
    Given a file "video.mp4" with source "https://example.com"
    When I generate show notes in "html" format
    Then the output contains "<a href"
    And the output contains "https://example.com"

  Scenario: Files without sources are excluded
    Given a file "with-source.mp4" with source "https://example.com"
    And a file "no-source.mp4" exists
    When I generate show notes in "markdown" format
    Then the output contains "with-source"
    And the output does not contain "no-source"

  Scenario: Empty playlist produces empty output
    Given no files in playlist
    When I generate show notes in "markdown" format
    Then the output is empty
