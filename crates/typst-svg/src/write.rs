use ecow::EcoString;
use typst_library::layout::Transform;
use typst_utils::ResolvedPicoStr;
use xmlwriter::XmlWriter;

use crate::DedupId;

pub struct SvgElem<'a> {
    xml: &'a mut XmlWriter,
}

impl<'a> SvgElem<'a> {
    pub fn new(xml: &'a mut XmlWriter, name: &str) -> Self {
        xml.start_element(name);
        Self { xml }
    }

    pub fn elem(&mut self, name: &str) -> SvgElem<'_> {
        SvgElem::new(self.xml, name)
    }

    /// Creates a [`LazySvgElem`].
    pub fn lazy_elem<'b>(&'b mut self, name: &'static str) -> LazySvgElem<'a, 'b> {
        LazySvgElem::new(self, name)
    }

    /// Write an SVG attribute.
    pub fn attr(&mut self, name: &str, value: impl SvgDisplay) -> &mut Self {
        self.attr_with(name, |attr| value.fmt(attr));
        self
    }

    /// Write an SVG attribute, to an [`SvgFormatter`].
    pub fn attr_with(
        &mut self,
        name: &str,
        fmt: impl FnOnce(&mut SvgFormatter),
    ) -> &mut Self {
        self.xml
            .write_attribute_raw(name, |buf| fmt(&mut SvgFormatter::new(buf)));
        self
    }

    pub fn with(&mut self, f: impl FnOnce(&mut Self)) -> &mut Self {
        f(self);
        self
    }
}

impl Drop for SvgElem<'_> {
    fn drop(&mut self) {
        self.xml.end_element();
    }
}

/// Allows deferring the creation of a children element.
pub struct LazySvgElem<'a, 'b> {
    inner: &'b mut SvgElem<'a>,
    initialized: bool,
    name: &'static str,
}

impl<'a, 'b> LazySvgElem<'a, 'b> {
    pub fn new(parent: &'b mut SvgElem<'a>, name: &'static str) -> Self {
        Self { inner: parent, initialized: false, name }
    }

    /// Initialize the child element, if not already present.
    pub fn init<'c>(&'c mut self) -> &'c mut SvgElem<'a> {
        if !self.initialized {
            self.inner.xml.start_element(self.name);
            self.initialized = true;
        }
        self.inner
    }

    /// Either get the child element, if it has been initialized, otherwise get
    /// the parent.
    pub fn lazy<'c>(&'c mut self) -> &'c mut SvgElem<'a> {
        self.inner
    }
}

impl Drop for LazySvgElem<'_, '_> {
    fn drop(&mut self) {
        if self.initialized {
            self.inner.xml.end_element();
        }
    }
}

pub trait SvgWrite: Sized {
    /// Write a string, escaping is handled by [`xmlwriter`].
    fn push_str(&mut self, value: &str);

    /// Write a character, escaping is handled by [`xmlwriter`].
    fn push_char(&mut self, value: char) {
        self.push_str(value.encode_utf8(&mut [0; 4]));
    }

    /// Write a number.
    fn push_num(&mut self, num: f64) {
        const ROUNDING_FACTOR: f64 = 10_u32.pow(9) as f64;

        // If the number is an integer, format it using `itoa`. This should be
        // faster and avoids the `.0`.
        if num == num as i64 as f64 {
            self.push_int(num as i64);
            return;
        }

        // Round numbers to the specified precision to make them more
        // deterministic and to reduce the file size.
        let num = (ROUNDING_FACTOR * num).round() / ROUNDING_FACTOR;

        let mut buf = ryu::Buffer::new();
        self.push_str(buf.format(num));
    }

    /// Write an integer.
    fn push_int(&mut self, num: i64) {
        let mut buf = itoa::Buffer::new();
        self.push_str(buf.format(num));
    }

    /// Convenience method to write a list of space separated numbers.
    fn push_nums(&mut self, nums: impl IntoIterator<Item = f64>) {
        for (i, num) in nums.into_iter().enumerate() {
            if i > 0 {
                self.push_str(" ");
            }
            self.push_num(num);
        }
    }

    /// Write a value that implements [`SvgDisplay`].
    fn push(&mut self, value: impl SvgDisplay) {
        value.fmt(self);
    }
}

pub struct SvgFormatter<'a, T = Vec<u8>> {
    buf: &'a mut T,
}

impl<'a, T> SvgFormatter<'a, T> {
    pub fn new(buf: &'a mut T) -> Self {
        Self { buf }
    }
}

impl SvgWrite for SvgFormatter<'_, Vec<u8>> {
    fn push_str(&mut self, value: &str) {
        self.buf.extend_from_slice(value.as_bytes());
    }
}

impl SvgWrite for SvgFormatter<'_, EcoString> {
    fn push_str(&mut self, value: &str) {
        self.buf.push_str(value);
    }
}

pub trait SvgDisplay {
    fn fmt(&self, f: &mut impl SvgWrite);
}

impl<T: SvgDisplay> SvgDisplay for &T {
    fn fmt(&self, f: &mut impl SvgWrite) {
        <T as SvgDisplay>::fmt(self, f)
    }
}

impl SvgDisplay for char {
    fn fmt(&self, f: &mut impl SvgWrite) {
        f.push_char(*self);
    }
}

impl SvgDisplay for &str {
    fn fmt(&self, f: &mut impl SvgWrite) {
        f.push_str(self);
    }
}

impl SvgDisplay for EcoString {
    fn fmt(&self, f: &mut impl SvgWrite) {
        f.push_str(self);
    }
}

impl SvgDisplay for ResolvedPicoStr {
    fn fmt(&self, f: &mut impl SvgWrite) {
        f.push_str(self);
    }
}

impl SvgDisplay for f64 {
    fn fmt(&self, f: &mut impl SvgWrite) {
        f.push_num(*self);
    }
}

/// Displays as an SVG transform. The exact representation is chosen based on
/// the specific transform. Either `matrix`, `scale`, or `translate` is used.
///
/// See https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/transform
pub struct SvgTransform(pub Transform);

impl SvgDisplay for SvgTransform {
    fn fmt(&self, f: &mut impl SvgWrite) {
        let sx = self.0.sx.get();
        let sy = self.0.sy.get();
        let kx = self.0.kx.get();
        let ky = self.0.ky.get();
        let tx = self.0.tx.to_pt();
        let ty = self.0.ty.to_pt();

        if self.0.is_only_scale() {
            f.push_str("scale(");
            if sx == sy {
                f.push_num(sx);
            } else {
                f.push_nums([sx, sy]);
            }
            f.push_str(")")
        } else if self.0.is_only_translate() {
            f.push_str("translate(");
            if ty == 0.0 {
                f.push_num(tx);
            } else {
                f.push_nums([tx, ty]);
            }
            f.push_str(")");
        } else {
            f.push_str("matrix(");
            f.push_nums([sx, ky, kx, sy, tx, ty]);
            f.push_str(")");
        }
    }
}

/// A SVG URL.
pub struct SvgUrl<T>(pub T);

impl SvgDisplay for SvgUrl<DedupId> {
    fn fmt(&self, f: &mut impl SvgWrite) {
        f.push_str("url(#");
        f.push(self.0);
        f.push_str(")");
    }
}

/// A referenced SVG ID.
pub struct SvgIdRef<T>(pub T);

impl SvgDisplay for SvgIdRef<DedupId> {
    fn fmt(&self, f: &mut impl SvgWrite) {
        f.push_str("#");
        f.push(self.0);
    }
}

impl<S: AsRef<str>> SvgDisplay for SvgIdRef<S> {
    fn fmt(&self, f: &mut impl SvgWrite) {
        f.push_str("#");
        f.push_str(self.0.as_ref());
    }
}
