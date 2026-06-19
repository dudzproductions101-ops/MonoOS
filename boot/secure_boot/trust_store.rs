        Ok(())
    }

    /// Look up an anchor by its SHA-256 fingerprint.
    pub fn find_by_fingerprint(&self, fp: &[u8; SHA256_LEN]) -> Option<&TrustAnchor> {
        for slot in &self.anchors[..self.count] {
            if let Some(a) = slot {
                if &a.fingerprint == fp {
                    return Some(a);
                }
            }
        }
        None
    }

    /// Return all active anchors as a slice.
    pub fn anchors(&self) -> impl Iterator<Item = &TrustAnchor> {
        self.anchors[..self.count]
            .iter()
            .filter_map(|s| s.as_deref())
    }

    /// Return the count of loaded anchors.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Return true if the provided rollback index is acceptable.
    pub fn rollback_index_ok(&self, index: u64) -> bool {
        index >= self.min_rollback_index
    }

    /// Update the minimum rollback index (called after reading fuse values).
    pub fn set_min_rollback(&mut self, idx: u64) {
        self.min_rollback_index = idx;
    }
}
