//! The `Signal` marker trait.
//!
//! Any `Send + Sync + 'static` type is a signal — the blanket impl
//! below provides this. `#[signal]` (from `ember-macros`) is a no-op
//! attribute that makes signal declarations greppable; the actual
//! functionality comes from this blanket impl and from `SignalTable`
//! in Phase 7.

pub trait Signal: Send + Sync + 'static {}

impl<T: Send + Sync + 'static> Signal for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_struct_is_a_signal() {
        struct MyEvent {
            _amount: u32,
        }
        fn assert_signal<T: Signal>() {}
        assert_signal::<MyEvent>();
    }

    #[test]
    fn a_primitive_is_a_signal() {
        fn assert_signal<T: Signal>() {}
        assert_signal::<u32>();
        assert_signal::<String>();
    }
}
