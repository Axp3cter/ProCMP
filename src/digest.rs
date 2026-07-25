//! Content-addressed digests, used as cache keys and plan fingerprints.

const NIBBLES: &[u8; 16] = b"0123456789abcdef";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize)]
#[serde(into = "String")]
pub struct Digest([u8; 32]);

impl Digest {
    pub fn hex(self) -> String {
        self.render(32)
    }

    /// The first 12 characters, for human-facing output.
    pub fn short(self) -> String {
        self.render(6)
    }

    pub fn bytes(self) -> [u8; 32] {
        self.0
    }

    fn render(self, bytes: usize) -> String {
        let mut out = String::with_capacity(bytes * 2);
        for byte in &self.0[..bytes] {
            out.push(NIBBLES[usize::from(byte >> 4)] as char);
            out.push(NIBBLES[usize::from(byte & 0x0f)] as char);
        }
        out
    }
}

impl From<Digest> for String {
    fn from(digest: Digest) -> Self {
        digest.hex()
    }
}

/// Accumulates labelled fields. Everything is length-prefixed, so `["ab", "c"]` and
/// `["a", "bc"]` cannot share a key.
#[derive(Debug, Default)]
pub struct Hasher(blake3::Hasher);

impl Hasher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn field(&mut self, label: &str, bytes: impl AsRef<[u8]>) -> &mut Self {
        let bytes = bytes.as_ref();
        self.0.update(&(label.len() as u64).to_le_bytes());
        self.0.update(label.as_bytes());
        self.0.update(&(bytes.len() as u64).to_le_bytes());
        self.0.update(bytes);
        self
    }

    /// Order-sensitive. The count goes in last, which is what lets this stream the
    /// items rather than collect them to learn their length.
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

    pub fn finish(&self) -> Digest {
        Digest(*self.0.finalize().as_bytes())
    }
}

pub fn of(bytes: impl AsRef<[u8]>) -> Digest {
    Digest(*blake3::hash(bytes.as_ref()).as_bytes())
}
