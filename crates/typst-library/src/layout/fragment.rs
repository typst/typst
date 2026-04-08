use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use crate::layout::Frame;

/// Trait for reading frames from external storage (e.g., disk).
/// Used by disk-backed Fragment to lazily read frames without holding all in memory.
pub trait FrameSource: Send + Sync {
    /// Number of frames available.
    fn len(&self) -> usize;
    /// Read frame at the given index.
    fn read_frame(&self, index: usize) -> std::io::Result<Frame>;
}

/// A partial layout result.
#[derive(Clone)]
pub struct Fragment(FragmentInner);

#[derive(Clone)]
enum FragmentInner {
    Memory(Vec<Frame>),
    External(Arc<dyn FrameSource>),
}

impl Fragment {
    /// Create a fragment from a single frame.
    pub fn frame(frame: Frame) -> Self {
        Self(FragmentInner::Memory(vec![frame]))
    }

    /// Create a fragment from multiple frames.
    pub fn frames(frames: Vec<Frame>) -> Self {
        Self(FragmentInner::Memory(frames))
    }

    /// Create a fragment backed by an external frame source (e.g., disk).
    /// Frames are read lazily one at a time during iteration.
    pub fn from_source(source: Arc<dyn FrameSource>) -> Self {
        Self(FragmentInner::External(source))
    }

    /// Return `true` if the length is 0.
    pub fn is_empty(&self) -> bool {
        match &self.0 {
            FragmentInner::Memory(frames) => frames.is_empty(),
            FragmentInner::External(source) => source.len() == 0,
        }
    }

    /// The number of frames in the fragment.
    pub fn len(&self) -> usize {
        match &self.0 {
            FragmentInner::Memory(frames) => frames.len(),
            FragmentInner::External(source) => source.len(),
        }
    }

    /// Extract the first and only frame.
    ///
    /// Panics if there are multiple frames.
    #[track_caller]
    pub fn into_frame(self) -> Frame {
        match self.0 {
            FragmentInner::Memory(frames) => {
                assert_eq!(frames.len(), 1, "expected exactly one frame");
                frames.into_iter().next().unwrap()
            }
            FragmentInner::External(source) => {
                assert_eq!(source.len(), 1, "expected exactly one frame");
                source.read_frame(0).expect("failed to read frame from source")
            }
        }
    }

    /// Extract the frames.
    pub fn into_frames(self) -> Vec<Frame> {
        match self.0 {
            FragmentInner::Memory(frames) => frames,
            FragmentInner::External(source) => {
                let len = source.len();
                let mut frames = Vec::with_capacity(len);
                for i in 0..len {
                    frames.push(
                        source
                            .read_frame(i)
                            .expect("failed to read frame from source"),
                    );
                }
                frames
            }
        }
    }

    /// Extract a slice with the contained frames.
    ///
    /// Panics for disk-backed fragments (only used in block.rs, never disk-backed).
    pub fn as_slice(&self) -> &[Frame] {
        match &self.0 {
            FragmentInner::Memory(frames) => frames,
            FragmentInner::External(_) => {
                panic!("as_slice() is not supported for disk-backed fragments")
            }
        }
    }

    /// Iterate over the contained frames.
    ///
    /// Panics for disk-backed fragments (only used in block.rs, never disk-backed).
    pub fn iter(&self) -> std::slice::Iter<'_, Frame> {
        match &self.0 {
            FragmentInner::Memory(frames) => frames.iter(),
            FragmentInner::External(_) => {
                panic!("iter() is not supported for disk-backed fragments")
            }
        }
    }

    /// Iterate over the contained frames mutably.
    ///
    /// Panics for disk-backed fragments (only used in block.rs, never disk-backed).
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Frame> {
        match &mut self.0 {
            FragmentInner::Memory(frames) => frames.iter_mut(),
            FragmentInner::External(_) => {
                panic!("iter_mut() is not supported for disk-backed fragments")
            }
        }
    }

    /// Returns `true` if this fragment is backed by external storage (e.g., disk).
    pub fn is_disk_backed(&self) -> bool {
        matches!(&self.0, FragmentInner::External(_))
    }

    /// Check if there are any non-empty frames.
    /// Works for both in-memory and disk-backed fragments.
    pub fn has_any_non_empty(&self) -> bool {
        match &self.0 {
            FragmentInner::Memory(frames) => frames.iter().any(|f| !f.is_empty()),
            FragmentInner::External(source) => source.len() > 0,
        }
    }

    /// Consume the fragment into an iterator starting at `start`.
    /// For disk-backed fragments, this skips without reading frames before `start`.
    pub fn into_iter_from(self, start: usize) -> FragmentIntoIter {
        match self.0 {
            FragmentInner::Memory(frames) => {
                let mut iter = frames.into_iter();
                // Advance past the first `start` elements (drops them)
                for _ in 0..start {
                    iter.next();
                }
                FragmentIntoIter::Memory(iter)
            }
            FragmentInner::External(source) => {
                let len = source.len();
                FragmentIntoIter::External {
                    source,
                    index: start,
                    len,
                }
            }
        }
    }

    /// Materialize a disk-backed fragment into memory by reading all frames.
    /// No-op for already in-memory fragments.
    pub fn materialize(&mut self) {
        if let FragmentInner::External(source) = &self.0 {
            let frames: Vec<Frame> = (0..source.len())
                .map(|i| source.read_frame(i).expect("failed to read frame from source"))
                .collect();
            self.0 = FragmentInner::Memory(frames);
        }
    }

    /// Replace a frame at the given index with an empty placeholder.
    /// Only works for in-memory fragments. For disk-backed, this is a no-op.
    pub fn clear_frame(&mut self, index: usize) {
        if let FragmentInner::Memory(frames) = &mut self.0 {
            if index < frames.len() {
                frames[index] = Frame::soft(crate::layout::Size::zero());
            }
        }
    }
}

impl Debug for Fragment {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match &self.0 {
            FragmentInner::Memory(frames) => match frames.as_slice() {
                [frame] => frame.fmt(f),
                frames => frames.fmt(f),
            },
            FragmentInner::External(source) => {
                write!(f, "Fragment(disk-backed, {} frames)", source.len())
            }
        }
    }
}

/// Iterator for consuming a Fragment frame by frame.
/// For in-memory fragments, yields frames from the Vec.
/// For disk-backed fragments, reads frames lazily from the source.
pub enum FragmentIntoIter {
    Memory(std::vec::IntoIter<Frame>),
    External {
        source: Arc<dyn FrameSource>,
        index: usize,
        len: usize,
    },
}

impl Iterator for FragmentIntoIter {
    type Item = Frame;

    fn next(&mut self) -> Option<Frame> {
        match self {
            Self::Memory(iter) => iter.next(),
            Self::External { source, index, len } => {
                if *index >= *len {
                    return None;
                }
                // Read one frame from disk; if IO fails, return None.
                let frame = source.read_frame(*index).ok()?;
                *index += 1;
                Some(frame)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Memory(iter) => iter.size_hint(),
            Self::External { index, len, .. } => {
                let remaining = len.saturating_sub(*index);
                (remaining, Some(remaining))
            }
        }
    }
}

impl ExactSizeIterator for FragmentIntoIter {}

impl IntoIterator for Fragment {
    type Item = Frame;
    type IntoIter = FragmentIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        match self.0 {
            FragmentInner::Memory(frames) => FragmentIntoIter::Memory(frames.into_iter()),
            FragmentInner::External(source) => {
                let len = source.len();
                FragmentIntoIter::External { source, index: 0, len }
            }
        }
    }
}

impl<'a> IntoIterator for &'a Fragment {
    type Item = &'a Frame;
    type IntoIter = std::slice::Iter<'a, Frame>;

    fn into_iter(self) -> Self::IntoIter {
        match &self.0 {
            FragmentInner::Memory(frames) => frames.iter(),
            FragmentInner::External(_) => {
                panic!("&Fragment iteration is not supported for disk-backed fragments")
            }
        }
    }
}

impl<'a> IntoIterator for &'a mut Fragment {
    type Item = &'a mut Frame;
    type IntoIter = std::slice::IterMut<'a, Frame>;

    fn into_iter(self) -> Self::IntoIter {
        match &mut self.0 {
            FragmentInner::Memory(frames) => frames.iter_mut(),
            FragmentInner::External(_) => {
                panic!("&mut Fragment iteration is not supported for disk-backed fragments")
            }
        }
    }
}
