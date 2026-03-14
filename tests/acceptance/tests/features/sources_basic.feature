Feature: Sources basic operations
  Users can manage source URLs for files.

  Scenario: Add source to file
    Given a file "video.mp4"
    When I add source "https://youtube.com/watch?v=abc" to "video.mp4"
    Then the file "video.mp4" has source "https://youtube.com/watch?v=abc"

  Scenario: Add multiple sources to same file
    Given a file "video.mp4" with source "https://youtube.com/watch?v=abc"
    When I add source "https://archive.org/details/video" to "video.mp4"
    Then the file "video.mp4" has source "https://youtube.com/watch?v=abc"
    And the file "video.mp4" has source "https://archive.org/details/video"

  Scenario: List sources for file with multiple sources
    Given a file "video.mp4" with source "https://first.com"
    And the file "video.mp4" has source "https://second.com"
    When I list sources for "video.mp4"
    Then the output contains "https://first.com"
    And the output contains "https://second.com"

  Scenario: List sources for file with no sources
    Given a file "video.mp4"
    When I list sources for "video.mp4"
    Then the output is empty

  Scenario: Edit sources replaces all sources
    Given a file "video.mp4" with source "https://old.com"
    When I edit sources for "video.mp4" with "https://new1.com\nhttps://new2.com"
    Then the file "video.mp4" has source "https://new1.com"
    And the file "video.mp4" has source "https://new2.com"
    And the file "video.mp4" does not have source "https://old.com"
