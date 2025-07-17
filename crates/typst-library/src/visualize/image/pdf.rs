use std::sync::Arc;
use crate::foundations::Bytes;

/// A PDF image.
#[derive(Clone, Hash)]
pub struct PdfImage(Bytes);