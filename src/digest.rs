//! Content-addressed digests, used as cache keys and as plan fingerprints.

/// A 32-byte BLAKE3 digest.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize)]
#[serde(into = "String")]
pub struct Digest([u8; 32]);

impl Digest {
    /// Returns the full 64-character lowercase hexadecimal form.
    pub fn hex(self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Returns the first 12 hexadecimal characters, for human-facing output.
    pub fn short(self) -> String {
        self.hex()[..12].to_owned()
    }

    pub fn bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl From<Digest> for String {
    fn from(digest: Digest) -> Self {
        digest.hex()
    }
}

/// Accumulates labelled fields into a [`Digest`].
///
/// Fields are length-prefixed, so `["ab", "c"]` and `["a", "bc"]` hash differently.
/// Without that, two configurations could share a cache key.
#[derive(Debug, Default)]
pub struct Hasher(blake3::Hasher);

impl Hasher {
    pub fn new() -> Self {
        Self::default()
    }

    /// The label is hashed too, so renaming a field invalidates the cache.
    pub fn field(&mut self, label: &str, bytes: impl AsRef<[u8]>) -> &mut Self {
        let bytes = bytes.as_ref();
        self.0.update(&(label.len() as u64).to_le_bytes());
        self.0.update(label.as_bytes());
        self.0.update(&(bytes.len() as u64).to_le_bytes());
        self.0.update(bytes);
        self
    }

    /// Order-sensitive: rule order changes build output, so it must change the key.
    pub fn seq<I, S>(&mut self, label: &str, items: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<[u8]>,
    {
        let items: Vec<S> = items.into_iter().collect();
        self.field(label, (items.len() as u64).to_le_bytes());
        for (index, item) in items.iter().enumerate() {
            self.0.update(&(index as u64).to_le_bytes());
            let bytes = item.as_ref();
            self.0.update(&(bytes.len() as u64).to_le_bytes());
            self.0.update(bytes);
        }
        self
    }

    pub fn finish(&self) -> Digest {
        Digest(*self.0.finalize().as_bytes())
    }
}

pub fn of(bytes: impl AsRef<[u8]>) -> Digest {
    Digest(*blake3::hash(bytes.as_ref()).as_bytes())
}
