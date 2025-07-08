use crate::foundations::{elem, Content};
use crate::layout::{Length, Rel};

/// Adds spacing around content.
///
/// The spacing can be specified for each side individually, or for all sides at
/// once by specifying a positional argument.
///
/// # Example
/// ```example
/// #set align(center)
///
/// #pad(x: 16pt, image("typing.jpg"))
/// _Typing speeds can be
///  measured in words per minute._
/// ```
#[elem(title = "Padding")]
pub struct PadElem {
    /// The padding at the left side.
    #[parse(
        let all = args.named("rest")?.or(args.find()?);
        let x = args.named("x")?.or(all);
        let y = args.named("y")?.or(all);
        args.named("left")?.or(x)
    )]
    pub left: Rel<Length>,

    /// The padding at the top side.
    #[parse(args.named("top")?.or(y))]
    pub top: Rel<Length>,

    /// The padding at the right side.
    #[parse(args.named("right")?.or(x))]
    pub right: Rel<Length>,

    /// The padding at the bottom side.
    #[parse(args.named("bottom")?.or(y))]
    pub bottom: Rel<Length>,

    /// A shorthand to set `left` and `right` to the same value.
    #[external]
    pub x: Rel<Length>,

    /// A shorthand to set `top` and `bottom` to the same value.
    #[external]
    pub y: Rel<Length>,

    /// A shorthand to set all four sides to the same value.
    #[external]
    pub rest: Rel<Length>,

    /// The content to pad at the sides.
    #[required]
    pub body: Content,
}
