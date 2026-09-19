use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::Zeroize;

/// Number of bytes reserved for a secret buffer
pub const SECRET_CAPACITY: usize = 512;

/// Plaintext authentication material
#[derive(PartialEq, Eq)]
pub struct Secret {
    value: String,
    char_count: usize,
}

impl Secret {
    pub fn new() -> Self {
        Self {
            value: String::with_capacity(SECRET_CAPACITY),
            char_count: 0,
        }
    }

    /// Returns the plaintext. Each call site is somewhere the secret can escape, so keep them few
    pub fn expose(&self) -> &str {
        &self.value
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    pub fn char_count(&self) -> usize {
        self.char_count
    }

    pub fn push(&mut self, character: char) {
        self.value.push(character);
        self.char_count = self.char_count.saturating_add(1);
    }

    pub fn pop(&mut self) {
        if self.value.pop().is_some() {
            self.char_count = self.char_count.saturating_sub(1);
        }
    }

    pub fn clear(&mut self) {
        self.value.zeroize();
        self.value.reserve(SECRET_CAPACITY);
        self.char_count = 0;
    }

    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }

    pub fn duplicate(&self) -> Self {
        let mut duplicate = Self::new();
        duplicate.value.push_str(&self.value);
        duplicate.char_count = self.char_count;
        duplicate
    }

    #[cfg(test)]
    fn capacity(&self) -> usize {
        self.value.capacity()
    }
}

impl Default for Secret {
    fn default() -> Self {
        Self::new()
    }
}

impl From<String> for Secret {
    fn from(mut value: String) -> Self {
        let mut secret = Self::new();
        secret.value.push_str(&value);
        secret.char_count = value.chars().count();
        value.zeroize();
        secret
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Secret(<redacted>)")
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        self.value.zeroize();
        self.char_count = 0;
    }
}

impl Serialize for Secret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.value)
    }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    use super::{SECRET_CAPACITY, Secret};

    #[test]
    fn redacts_plaintext_in_debug_output() {
        let secret = Secret::from(String::from("hunter2"));

        let rendered = format!("{secret:?}");

        assert!(
            !rendered.contains("hunter2"),
            "debug output leaked: {rendered}"
        );
        assert_eq!(rendered, "Secret(<redacted>)");
    }

    #[test]
    fn reserves_enough_capacity_to_never_reallocate_while_typing() {
        let mut secret = Secret::new();
        let capacity_before = secret.capacity();

        // The input layer caps entry at 128 characters // 4-byte characters are the worst case
        for _ in 0..128 {
            secret.push('\u{10348}');
        }

        assert_eq!(secret.capacity(), capacity_before);
        assert!(capacity_before >= SECRET_CAPACITY);
    }

    #[test]
    fn keeps_capacity_after_clearing() {
        let mut secret = Secret::from(String::from("hunter2"));
        secret.clear();

        assert!(secret.is_empty());
        assert!(secret.capacity() >= SECRET_CAPACITY);
    }

    #[test]
    fn pop_removes_only_the_last_character() {
        let mut secret = Secret::new();
        for character in "abcä".chars() {
            secret.push(character);
        }

        secret.pop();

        assert_eq!(secret.expose(), "abc");
        assert_eq!(secret.char_count(), 3);
    }

    #[test]
    fn take_moves_the_secret_and_replaces_it_with_a_reserved_empty_buffer() {
        let mut secret = Secret::from(String::from("hunter2"));

        let taken = secret.take();

        assert_eq!(taken.expose(), "hunter2");
        assert!(secret.is_empty());
        assert_eq!(secret.char_count(), 0);
        assert!(secret.capacity() >= SECRET_CAPACITY);
    }

    #[test]
    fn duplicate_is_an_explicit_owned_copy() {
        let secret = Secret::from(String::from("pāssword"));

        let duplicate = secret.duplicate();

        assert_eq!(duplicate.expose(), secret.expose());
        assert_eq!(duplicate.char_count(), 8);
    }

    #[test]
    fn round_trips_through_serde() {
        let secret = Secret::from(String::from("hunter2"));
        let encoded = serde_json::to_string(&secret).expect("encode");
        let decoded: Secret = serde_json::from_str(&encoded).expect("decode");

        assert_eq!(encoded, "\"hunter2\"");
        assert_eq!(decoded.expose(), "hunter2");
    }
}
