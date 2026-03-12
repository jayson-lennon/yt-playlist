Feature: Alias Deletion
  When an alias is removed from a file, it should be deleted from storage.

  Scenario: Removing an alias deletes it from storage
    Given a real file at "video.mp4"
    And the file "video.mp4" has alias "My Video"
    When I remove the alias from "video.mp4"
    Then the file "video.mp4" has no alias

  Scenario: Removing non-existent alias succeeds
    Given a real file at "video.mp4"
    When I remove the alias from "video.mp4"
    Then the file "video.mp4" has no alias
