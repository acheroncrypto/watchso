//! Utilities for [`Action`].

use std::{collections::HashSet, path::Path};

use watchexec::{action::Action, event::Event, signal::source::MainSignal};

/// Utility struct for [`Action`].
pub struct WAction(Action);

impl WAction {
    /// Create a new [`WAction`].
    pub fn new(action: Action) -> Self {
        Self(action)
    }

    /// Return the internal action by consuming `self`.
    pub fn take(self) -> Action {
        self.0
    }

    /// Returns whether the action includes [`MainSignal::Interrupt`].
    pub fn is_interrupt(&self) -> bool {
        self.is_any_signal(MainSignal::Interrupt)
    }

    /// Returns whether the action includes [`MainSignal::Terminate`].
    pub fn is_terminate(&self) -> bool {
        self.is_any_signal(MainSignal::Terminate)
    }

    /// Get all the unique paths in the action event paths.
    pub fn get_unique_paths(&self) -> HashSet<&Path> {
        let mut hashset = HashSet::new();
        for (path, _) in self.0.events.iter().flat_map(Event::paths) {
            hashset.insert(path);
        }
        hashset
    }

    /// Returns whether any signal includes the given signal in the events list.
    fn is_any_signal(&self, signal: MainSignal) -> bool {
        self.0
            .events
            .iter()
            .flat_map(Event::signals)
            .any(|sig| sig == signal)
    }
}
