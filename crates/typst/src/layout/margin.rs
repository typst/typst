use crate::diag::{bail, SourceResult, StrResult};
use crate::foundations::{
    cast, func, scope, ty, Args, AutoValue, Dict, Fold, FromValue, IntoValue, Repr,
    Resolve, Smart, StyleChain, Value,
};
use crate::layout::{Length, Rel, Sides};
use ecow::{eco_format, EcoString};

type MarginLength = Option<Smart<Rel<Length>>>;

/// Defines a page's margin.
///
/// Specification of a margin.
///
/// A margin has four components: left, top, right, bottom. To construct a
/// `margin` you may provide multiple forms of arguments:
///
/// - `{auto}`: The margins are set automatically to 2.5/21 times the smaller
///   dimension of the page. This results in 2.5cm margins for an A4 page.
/// - A single length: The same margin on all sides.
/// - A dictionary: With a dictionary, the margins can be set individually.
///   The dictionary can contain the following keys in order of precedence:
///   - `left`: The left margin.
///   - `top`: The top margin.
///   - `right`: The right margin.
///   - `bottom`: The bottom margin.
///   - `inside`: The margin at the inner side of the page (where the
///     [binding]($page.binding) is).
///   - `outside`: The margin at the outer side of the page (opposite to the
///     [binding]($page.binding)).
///   - `x`: The horizontal margins.
///   - `y`: The vertical margins.
///   - `rest`: The margins on all sides except those for which the
///     dictionary explicitly sets a size.
///
/// The values for `left` and `right` are mutually exclusive with
/// the values for `inside` and `outside`.
///
/// You can provide a `{margin}` object to any function that expects a margin.
/// Also, on a `margin` object, you can access the fields of top, bottom, left,
/// right, whose value is [context]($context)-dependent. For example,
/// `context margin(inside: 1em).left` returns the left margin calculated from
/// the context.
#[ty(scope, cast)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Margina {
    /// The margins for each side.
    pub sides: Sides<MarginLength>,
    /// Whether to swap `left` and `right` to make them `inside` and `outside`
    /// (when to swap depends on the binding).
    pub two_sided: Option<bool>,
}

impl Margina {
    /// Create an instance with four equal components.
    pub fn splat(value: MarginLength) -> Self {
        Self { sides: Sides::splat(value), two_sided: None }
    }
}

#[scope]
impl Margina {
    #[func(constructor)]
    pub fn construct(
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: &mut Args,

        /// Applies the `{auto}` value to all sides. The margins are set
        /// automatically to 2.5/21 times the smaller dimension of the
        /// page. This results in 2.5cm margins for an A4 page.
        #[external]
        auto_value: AutoValue,

        /// Applies a relative length to all sides.
        #[external]
        length: Rel<Length>,

        /// Applies a dictionary to set the margins individually. It can contain
        /// the following keys in order of precedence:
        ///   - `top`: The top margin.
        ///   - `right`: The right margin.
        ///   - `bottom`: The bottom margin.
        ///   - `left`: The left margin.
        ///   - `inside`: The margin at the inner side of the page (where the
        ///     [binding]($page.binding) is).
        ///   - `outside`: The margin at the outer side of the page (opposite to
        ///     the [binding]($page.binding)).
        ///   - `x`: The horizontal margins.
        ///   - `y`: The vertical margins.
        ///   - `rest`: The margins on all sides except those for which the
        ///     dictionary explicitly sets a size.
        ///
        /// The values for `left` and `right` are mutually exclusive with
        /// the values for `inside` and `outside`.
        #[external]
        dict: Dict,
    ) -> SourceResult<Margina> {
        if let Some(margin) = args.eat::<Margina>()? {
            return Ok(margin);
        }

        if let Some(_) = args.eat::<AutoValue>()? {
            return Ok(Self::splat(Some(Smart::Auto)));
        }

        if let Some(v) = args.eat::<Rel<Length>>()? {
            return Ok(Self::splat(Some(Smart::Custom(v))));
        }

        fn take(args: &mut Args, arg: &str) -> SourceResult<MarginLength> {
            Ok(args.named::<Smart<Rel<Length>>>(arg)?)
        }

        let rest = take(args, "rest")?;
        let x = take(args, "x")?.or(rest);
        let y = take(args, "y")?.or(rest);
        let top = take(args, "top")?.or(y);
        let bottom = take(args, "bottom")?.or(y);
        let outside = take(args, "outside")?;
        let inside = take(args, "inside")?;
        let left = take(args, "left")?;
        let right = take(args, "right")?;

        let implicitly_two_sided = outside.is_some() || inside.is_some();
        let implicitly_not_two_sided = left.is_some() || right.is_some();
        if implicitly_two_sided && implicitly_not_two_sided {
            bail!(
                args.span,
                "`inside` and `outside` are mutually exclusive with `left` and `right`"
            );
        }

        // - If 'implicitly_two_sided' is false here, then
        //   'implicitly_not_two_sided' will be guaranteed to be true
        //    due to the previous two 'if' conditions.
        // - If both are false, this means that this margin change does not
        //   affect lateral margins, and thus shouldn't make a difference on
        //   the 'two_sided' attribute of this margin.
        let two_sided = (implicitly_two_sided || implicitly_not_two_sided)
            .then_some(implicitly_two_sided);

        return Ok(Self {
            sides: Sides {
                left: inside.or(left).or(x),
                top,
                right: outside.or(right).or(x),
                bottom,
            },
            two_sided,
        });
    }

    #[func]
    pub fn left(&self) -> MarginLength {
        self.sides.left
    }

    #[func]
    pub fn top(&self) -> MarginLength {
        self.sides.top
    }

    #[func]
    pub fn right(&self) -> MarginLength {
        self.sides.right
    }

    #[func]
    pub fn bottom(&self) -> MarginLength {
        self.sides.bottom
    }
}

impl Repr for Margina {
    fn repr(&self) -> EcoString {
        eco_format!("margin()") // TODO
    }
}

impl Default for Margina {
    fn default() -> Self {
        Self {
            sides: Sides::splat(Some(Smart::Auto)),
            two_sided: None,
        }
    }
}

impl Fold for Margina {
    fn fold(self, outer: Self) -> Self {
        Self {
            sides: self.sides.fold(outer.sides),
            two_sided: self.two_sided.fold(outer.two_sided),
        }
    }
}

impl Resolve for Margina {
    type Output = Margina;

    fn resolve(self, _: StyleChain) -> Self::Output {
        Self { sides: self.sides, two_sided: self.two_sided }
    }
}

// Specifies a margin.
cast! {
    Margina,
    self => {
        let two_sided = self.two_sided.unwrap_or(false);
        if !two_sided && self.sides.is_uniform() {
            if let Some(left) = self.sides.left {
                return left.into_value();
            }
        }

        let mut dict = Dict::new();
        let mut handle = |key: &str, component: Option<Smart<Rel<Length>>>| {
            if let Some(c) = component {
                dict.insert(key.into(), c.into_value());
            }
        };

        handle("top", self.sides.top);
        handle("bottom", self.sides.bottom);
        if two_sided {
            handle("inside", self.sides.left);
            handle("outside", self.sides.right);
        } else {
            handle("left", self.sides.left);
            handle("right", self.sides.right);
        }

        Value::Dict(dict)
    },
    _: AutoValue => Self::splat(Some(Smart::Auto)),
    v: Rel<Length> => Self::splat(Some(Smart::Custom(v))),
    mut dict: Dict => {
        let mut take = |key| dict.take(key).ok().map(Value::cast).transpose();

        let rest = take("rest")?;
        let x = take("x")?.or(rest);
        let y = take("y")?.or(rest);
        let top = take("top")?.or(y);
        let bottom = take("bottom")?.or(y);
        let outside = take("outside")?;
        let inside = take("inside")?;
        let left = take("left")?;
        let right = take("right")?;

        let implicitly_two_sided = outside.is_some() || inside.is_some();
        let implicitly_not_two_sided = left.is_some() || right.is_some();
        if implicitly_two_sided && implicitly_not_two_sided {
            bail!("`inside` and `outside` are mutually exclusive with `left` and `right`");
        }

        // - If 'implicitly_two_sided' is false here, then
        //   'implicitly_not_two_sided' will be guaranteed to be true
        //    due to the previous two 'if' conditions.
        // - If both are false, this means that this margin change does not
        //   affect lateral margins, and thus shouldn't make a difference on
        //   the 'two_sided' attribute of this margin.
        let two_sided = (implicitly_two_sided || implicitly_not_two_sided)
            .then_some(implicitly_two_sided);

        dict.finish(&[
            "left", "top", "right", "bottom", "outside", "inside", "x", "y", "rest",
        ])?;

        Self {
            sides: Sides {
                left: inside.or(left).or(x),
                top,
                right: outside.or(right).or(x),
                bottom,
            },
            two_sided,
        }
    }
}
