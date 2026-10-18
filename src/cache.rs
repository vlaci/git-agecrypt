use std::path::Path;

const MAGIC: &[u8; 8] = b"GACACHE1";

pub(crate) const UNKNOWN_RECIPIENTS: [u8; 32] = [0x00; 32];
pub(crate) const FORCE_REENCRYPT: [u8; 32] = [0xff; 32];

pub(crate) struct CacheEntry {
    pub plaintext_hash: [u8; 32],
    pub recipients: [u8; 32],
    pub ciphertext: Vec<u8>,
}

impl CacheEntry {
    pub fn force_reencrypt() -> Self {
        Self {
            plaintext_hash: [0; 32],
            recipients: FORCE_REENCRYPT,
            ciphertext: vec![],
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(8 + 32 + 32 + self.ciphertext.len());
        buf.extend_from_slice(MAGIC);
        buf.extend_from_slice(&self.plaintext_hash);
        buf.extend_from_slice(&self.recipients);
        buf.extend_from_slice(&self.ciphertext);
        buf
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 72 || &bytes[..8] != MAGIC {
            return None;
        }
        let mut plaintext_hash = [0u8; 32];
        plaintext_hash.copy_from_slice(&bytes[8..40]);
        let mut recipients = [0u8; 32];
        recipients.copy_from_slice(&bytes[40..72]);
        let ciphertext = bytes[72..].to_vec();
        Some(Self {
            plaintext_hash,
            recipients,
            ciphertext,
        })
    }
}

#[allow(dead_code)]
pub(crate) fn recipients_fingerprint(recipients: &[String]) -> [u8; 32] {
    let mut v: Vec<&str> = recipients
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    v.sort_unstable();
    v.dedup();
    let joined = v.join("\n");
    let hash = blake3::hash(joined.as_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    out
}

pub(crate) fn file_name(relpath: &Path) -> String {
    #[cfg(unix)]
    let bytes = std::os::unix::ffi::OsStrExt::as_bytes(relpath.as_os_str());
    #[cfg(not(unix))]
    let bytes = relpath.to_string_lossy().as_bytes();

    blake3::hash(bytes).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let entry = CacheEntry {
            plaintext_hash: [1u8; 32],
            recipients: [2u8; 32],
            ciphertext: b"hello world".to_vec(),
        };
        let encoded = entry.encode();
        let decoded = CacheEntry::decode(&encoded).unwrap();
        assert_eq!(decoded.plaintext_hash, entry.plaintext_hash);
        assert_eq!(decoded.recipients, entry.recipients);
        assert_eq!(decoded.ciphertext, entry.ciphertext);
    }

    #[test]
    fn decode_short_rejects() {
        let short = [0u8; 71];
        assert!(CacheEntry::decode(&short).is_none());
    }

    #[test]
    fn decode_wrong_magic_rejects() {
        let mut bytes = vec![0u8; 72];
        bytes[..8].copy_from_slice(b"BADMAGIC");
        assert!(CacheEntry::decode(&bytes).is_none());
    }

    #[test]
    fn recipients_fingerprint_sorts_and_dedups() {
        let fp1 = recipients_fingerprint(&["b".into(), "a".into()]);
        let fp2 = recipients_fingerprint(&[" a".into(), "b".into(), "a".into()]);
        assert_eq!(fp1, fp2);
        assert_ne!(fp1, recipients_fingerprint(&["a".into()]));
    }
}
