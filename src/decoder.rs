use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    PassThrough,
    Consume,
    Launch(String),
}

pub struct Decoder {
    leader: u32,
    timeout: Duration,
    mappings: HashMap<u32, String>,
    deadline: Option<Instant>,
    consumed_keys: HashSet<u32>,
}

impl Decoder {
    pub fn new(leader: u32, timeout_ms: u64, mappings: HashMap<u32, String>) -> Self {
        Self {
            leader,
            timeout: Duration::from_millis(timeout_ms),
            mappings,
            deadline: None,
            consumed_keys: HashSet::new(),
        }
    }

    pub fn key_down(&mut self, key: u32, modified: bool, now: Instant) -> Action {
        if self.consumed_keys.contains(&key) {
            return Action::Consume;
        }
        if modified {
            self.deadline = None;
            return Action::PassThrough;
        }
        if self.deadline.is_some_and(|deadline| now > deadline) {
            self.deadline = None;
        }
        if self.deadline.is_none() {
            if key == self.leader {
                self.deadline = Some(now + self.timeout);
                self.consumed_keys.insert(key);
                Action::Consume
            } else {
                Action::PassThrough
            }
        } else if key == self.leader {
            self.deadline = Some(now + self.timeout);
            self.consumed_keys.insert(key);
            Action::Consume
        } else {
            self.deadline = None;
            if let Some(target) = self.mappings.get(&key).cloned() {
                self.consumed_keys.insert(key);
                Action::Launch(target)
            } else {
                Action::PassThrough
            }
        }
    }

    pub fn key_up(&mut self, key: u32) -> bool {
        self.consumed_keys.remove(&key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decoder() -> Decoder {
        Decoder::new(0x14, 600, HashMap::from([(u32::from(b'G'), "target".into())]))
    }

    #[test]
    fn leader_then_mapping_launches() {
        let now = Instant::now();
        let mut decoder = decoder();
        assert_eq!(decoder.key_down(0x14, false, now), Action::Consume);
        assert_eq!(decoder.key_down(u32::from(b'G'), false, now), Action::Launch("target".into()));
    }

    #[test]
    fn unknown_and_expired_sequences_pass_through() {
        let now = Instant::now();
        let mut decoder = decoder();
        decoder.key_down(0x14, false, now);
        assert_eq!(decoder.key_down(u32::from(b'X'), false, now), Action::PassThrough);
        decoder.key_down(0x14, false, now);
        assert_eq!(
            decoder.key_down(u32::from(b'G'), false, now + Duration::from_millis(601)),
            Action::PassThrough
        );
    }

    #[test]
    fn modified_input_cancels_sequence() {
        let now = Instant::now();
        let mut decoder = decoder();
        decoder.key_down(0x14, false, now);
        assert_eq!(decoder.key_down(u32::from(b'G'), true, now), Action::PassThrough);
        assert_eq!(decoder.key_down(u32::from(b'G'), false, now), Action::PassThrough);
    }

    #[test]
    fn leader_key_up_matches_key_down_handling() {
        let now = Instant::now();
        let mut decoder = decoder();
        decoder.key_down(0x14, true, now);
        assert!(!decoder.key_up(0x14));
        decoder.key_down(0x14, false, now);
        assert!(decoder.key_up(0x14));
    }

    #[test]
    fn mapping_repeat_and_key_up_are_consumed() {
        let now = Instant::now();
        let mut decoder = decoder();
        decoder.key_down(0x14, false, now);
        decoder.key_up(0x14);
        assert_eq!(decoder.key_down(u32::from(b'G'), false, now), Action::Launch("target".into()));
        assert_eq!(decoder.key_down(u32::from(b'G'), false, now), Action::Consume);
        assert!(decoder.key_up(u32::from(b'G')));
        assert_eq!(decoder.key_down(u32::from(b'G'), false, now), Action::PassThrough);
    }
}
