//! Own the resource bundle until every game borrowing it has been dropped.
use super::{Resources, split::Split};
use std::sync::Arc;

type Games<'a> = Option<Split<'a>>;

self_cell::self_cell! {
    pub(super) struct Session {
        owner: Arc<Resources>,
        #[not_covariant]
        dependent: Games,
    }
}

impl Session {
    pub(super) fn has_game(&self) -> bool {
        self.with_dependent(|_, game| game.is_some())
    }

    pub(super) fn with_game<R>(&mut self, f: impl FnOnce(&mut Split<'_>) -> R) -> R {
        self.with_dependent_mut(|_, game| f(game.as_mut().expect("ready session has a game")))
    }
}
