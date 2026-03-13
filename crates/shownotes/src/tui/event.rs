/// Result of a component handling a key event.
///
/// Used to implement event bubbling - when a component returns `Consumed`,
/// the event stops propagating. When it returns `Ignored`, the event
/// continues to the next handler in the chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    /// The event was consumed and should not propagate further.
    Consumed,
    /// The event was ignored and should bubble to the next handler.
    Ignored,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_result_consumed_is_distinct() {
        let consumed = EventResult::Consumed;
        let ignored = EventResult::Ignored;

        assert_ne!(consumed, ignored);
    }

    #[test]
    fn event_result_can_be_copied() {
        let result = EventResult::Consumed;
        let copied = result;

        assert_eq!(result, copied);
    }

    #[test]
    fn event_result_debug_formats_correctly() {
        assert_eq!(format!("{:?}", EventResult::Consumed), "Consumed");
        assert_eq!(format!("{:?}", EventResult::Ignored), "Ignored");
    }

    #[test]
    fn event_result_clone_works() {
        let result = EventResult::Consumed;
        let cloned = result;

        assert_eq!(result, cloned);
    }
}
