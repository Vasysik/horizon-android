//! One background search, latest pending request, and a small short-lived cache.
use std::time::{Duration, Instant};

pub struct Queue<K, V> {
    active: Option<K>,
    wanted: Option<K>,
    cache: Vec<(K, Instant, V)>,
}
impl<K: Clone + PartialEq, V: Clone> Queue<K, V> {
    pub fn new() -> Self {
        Self {
            active: None,
            wanted: None,
            cache: vec![],
        }
    }
    pub fn request(&mut self, key: K, refresh: bool) -> Option<V> {
        self.cache
            .retain(|(k, at, _)| (!refresh || *k != key) && at.elapsed() < Duration::from_secs(30));
        if let Some((_, _, value)) = self.cache.iter().find(|(k, _, _)| *k == key) {
            self.wanted = None;
            return Some(value.clone());
        }
        self.wanted = Some(key);
        None
    }
    pub fn take_next(&mut self) -> Option<K> {
        if self.active.is_some() {
            return None;
        }
        let key = self.wanted.as_ref()?.clone();
        self.active = Some(key.clone());
        Some(key)
    }
    /// Cache stale results, but only the latest requested result may update the UI.
    pub fn finish(&mut self, value: Option<V>) -> bool {
        let Some(key) = self.active.take() else {
            return false;
        };
        let visible = self.wanted.as_ref() == Some(&key);
        if visible {
            self.wanted = None;
        }
        if let Some(value) = value {
            self.cache.retain(|(k, _, _)| *k != key);
            if self.cache.len() >= 8 {
                self.cache.remove(0);
            }
            self.cache.push((key, Instant::now(), value));
        }
        visible
    }
    pub fn cancel_interest(&mut self) {
        self.wanted = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rapid_tabs_keep_one_search_and_only_the_latest_pending_request() {
        let mut q = Queue::new();
        assert_eq!(q.request("mods", false), None);
        assert_eq!(q.take_next(), Some("mods"));
        q.request("packs", false);
        q.request("shaders", false);
        assert_eq!(q.take_next(), None);
        assert!(!q.finish(Some(vec![1])));
        assert_eq!(q.take_next(), Some("shaders"));
        assert!(q.finish(Some(vec![2])));
        assert_eq!(q.take_next(), None);
        assert_eq!(q.request("mods", false), Some(vec![1]));
        assert_eq!(q.take_next(), None);
    }
    #[test]
    fn shader_origins_keep_separate_results_and_reject_a_late_reply() {
        let mut q = Queue::new();
        let horizon = ("shaders", "horizon", "water", 0);
        let modrinth = ("shaders", "modrinth", "water", 0);
        q.request(horizon, false);
        assert_eq!(q.take_next(), Some(horizon));
        q.request(modrinth, false);
        assert!(!q.finish(Some(vec!["native"])));
        assert_eq!(q.take_next(), Some(modrinth));
        assert!(q.finish(Some(vec!["classic"])));
        assert_eq!(q.request(horizon, false), Some(vec!["native"]));
        assert_eq!(q.request(modrinth, false), Some(vec!["classic"]));
    }
    #[test]
    fn explicit_refresh_bypasses_cache_and_errors_do_not_retry_forever() {
        let mut q = Queue::new();
        q.request(1, false);
        q.take_next();
        q.finish(Some(9));
        assert_eq!(q.request(1, false), Some(9));
        assert_eq!(q.request(1, true), None);
        assert_eq!(q.take_next(), Some(1));
        assert!(q.finish(None));
        assert_eq!(q.take_next(), None);
    }
    #[test]
    fn leaving_catalog_prevents_late_navigation_and_cache_is_bounded() {
        let mut q = Queue::new();
        q.request(0, false);
        q.take_next();
        q.cancel_interest();
        assert!(!q.finish(Some(0)));
        for key in 1..20 {
            q.request(key, false);
            q.take_next();
            q.finish(Some(key));
        }
        assert_eq!(q.request(19, false), Some(19));
        assert_eq!(q.request(0, false), None);
        assert!(q.cache.len() <= 8);
    }
    #[test]
    fn expired_query_is_fetched_again() {
        let mut q = Queue::new();
        q.request(1, false);
        q.take_next();
        q.finish(Some(9));
        q.cache[0].1 = Instant::now() - Duration::from_secs(31);
        assert_eq!(q.request(1, false), None);
        assert_eq!(q.take_next(), Some(1));
    }
}
