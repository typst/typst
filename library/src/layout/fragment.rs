use crate::prelude::*;

/// A partial layout result.
#[derive(Clone)]
pub struct Fragment(Vec<Frame>);

impl Fragment {
    /// Create a fragment from a single frame.
    #[inline]
    #[must_use]
    pub fn frame(frame: Frame) -> Self {
        Self(vec![frame])
    }

    /// Create a fragment from multiple frames.
    #[inline]
    #[must_use]
    pub fn frames(frames: Vec<Frame>) -> Self {
        Self(frames)
    }

    /// The number of frames in the fragment.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the fragment has no frames.
    ///
    /// Almost definitely should not occur in normal usage.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Extract the first and only frame.
    ///
    /// # Panics
    ///
    /// If there are multiple frames.
    #[track_caller]
    #[inline]
    #[must_use]
    pub fn into_frame(self) -> Frame {
        assert_eq!(self.0.len(), 1, "expected exactly one frame");
        self.0.into_iter().next().unwrap()
    }

    /// Extract the frames.
    #[inline]
    #[must_use]
    pub fn into_frames(self) -> Vec<Frame> {
        self.0
    }

    /// Iterate over the contained frames.
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, Frame> {
        self.0.iter()
    }

    /// Iterate over the contained frames.
    #[inline]
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Frame> {
        self.0.iter_mut()
    }
}

impl Debug for Fragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0.as_slice() {
            [frame] => frame.fmt(f),
            frames => frames.fmt(f),
        }
    }
}

impl IntoIterator for Fragment {
    type Item = Frame;
    type IntoIter = std::vec::IntoIter<Frame>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Fragment {
    type Item = &'a Frame;
    type IntoIter = std::slice::Iter<'a, Frame>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut Fragment {
    type Item = &'a mut Frame;
    type IntoIter = std::slice::IterMut<'a, Frame>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}
