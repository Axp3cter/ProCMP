//! Content-addressed digests, used as cache keys and as plan fingerprints.

/// A 32-byte BLAKE3 digest.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize)]
#[serde(into = "String")]
pub struct Digest([u8; 32]);

/// Lowercase hexadecimal, indexed by nibble.
const NIBBLES: &[u8; 16] = b"0123456789abcdef";

impl Digest {
    /// Returns the full 64-character lowercase hexadecimal form.
    ///
    /// Written a nibble at a time rather than through `format!`, because
    /// [`crate::engine::Scope::fingerprint`] renders one of these per source file and
    /// a formatter call per byte dominated that walk.
    pub fn hex(self) -> String {
        self.render(self.0.len())
    }

    /// Returns the first 12 hexadecimal characters, for human-facing output.
    pub fn short(self) -> String {
        self.render(6)
    }

    /// Renders the first `bytes` of the digest.
    fn render(self, bytes: usize) -> String {
        let mut out = String::with_capacity(bytes * 2);
        for byte in &self.0[..bytes] {
            out.push(NIBBLES[usize::from(byte >> 4)] as char);
            out.push(NIBBLES[usize::from(byte & 0x0f)] as char);
        }
        out
    }

    /// The raw bytes, for feeding one digest into another.
    pub fn bytes(self) -> [u8; 32] {
        self.0
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
    /// An empty hasher.
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
    ///
    /// The count is hashed last rather than first, which is what lets this stream the
    /// items instead of collecting them to learn their length. Position and length are
    /// still mixed in per item, so no reordering or resplitting collides.
    pub fn seq<I, S>(&mut self, label: &str, items: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<[u8]>,
    {
        let mut count: u64 = 0;
        for (index, item) in items.into_iter().enumerate() {
            self.0.update(&(index as u64).to_le_bytes());
            let bytes = item.as_ref();
            self.0.update(&(bytes.len() as u64).to_le_bytes());
            self.0.update(bytes);
            count += 1;
        }
        self.field(label, count.to_le_bytes())
    }

    /// The digest of everything fed in so far.
    pub fn finish(&self) -> Digest {
        Digest(*self.0.finalize().as_bytes())
    }
}

/// Hashes a single value, for a file's contents or a task identifier.
pub fn of(bytes: impl AsRef<[u8]>) -> Digest {
    Digest(*blake3::hash(bytes.as_ref()).as_bytes())
}
