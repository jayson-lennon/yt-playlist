Feature: Playlist persistence
  Playlist state is saved and loaded correctly.

  Scenario: Save and load playlist preserves order
    Given a playlist with files "first.mp4", "second.mp4", "third.mp4"
    When I save the playlist
    And I load the playlist
    Then the playlist contains "first.mp4", "second.mp4", "third.mp4" in order

  Scenario: Save preserves aliases
    Given a file "video.mp4" in playlist with alias "My Video"
    When I save the playlist
    And I load the playlist
    Then the file "video.mp4" has alias "My Video"

  Scenario: Empty playlist saves and loads correctly
    Given an empty playlist
    When I save the playlist
    And I load the playlist
    Then the playlist is empty

  Scenario: Reorder and save persists new order
    Given a playlist with files "a.mp4", "b.mp4", "c.mp4"
    When I reorder to "c.mp4", "a.mp4", "b.mp4"
    And I save the playlist
    And I load the playlist
    Then the playlist contains "c.mp4", "a.mp4", "b.mp4" in order
