use super::*;
use AlignmentKey::*;


/// An argument key which describes a target alignment.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AlignmentKey {
    Align(Alignment),
    Left,
    Top,
    Right,
    Bottom,
}

impl AlignmentKey {
    /// The generic axis this alignment key corresponds to in the given system
    /// of layouting axes. `None` if the alignment is generic.
    pub fn axis(self, axes: LayoutAxes) -> Option<GenericAxis> {
        match self {
            Left | Right => Some(Horizontal.to_generic(axes)),
            Top | Bottom => Some(Vertical.to_generic(axes)),
            Align(_) => None,
        }
    }

    /// The generic version of this alignment in the given system of layouting
    /// axes.
    ///
    /// Returns an error if the alignment is invalid for the given axis.
    pub fn to_generic(self, axes: LayoutAxes, axis: GenericAxis) -> LayoutResult<Alignment> {
        let specific = axis.to_specific(axes);
        let start = match axes.get(axis).is_positive() {
            true => Origin,
            false => End,
        };

        Ok(match (self, specific) {
            (Align(alignment), _) => alignment,
            (Left, Horizontal) | (Top, Vertical) => start,
            (Right, Horizontal) | (Bottom, Vertical) => start.inv(),

            _ => error!(
                "invalid alignment `{}` for {} axis",
                format!("{:?}", self).to_lowercase(),
                format!("{:?}", axis).to_lowercase()
            )
        })
    }

    /// The specific version of this alignment in the given system of layouting
    /// axes.
    pub fn to_specific(self, axes: LayoutAxes, axis: SpecificAxis) -> AlignmentKey {
        let direction = axes.get_specific(axis);
        if let Align(alignment) = self {
            match (direction, alignment) {
                (LeftToRight, Origin) | (RightToLeft, End) => Left,
                (LeftToRight, End) | (RightToLeft, Origin) => Right,
                (TopToBottom, Origin) | (BottomToTop, End) => Top,
                (TopToBottom, End) | (BottomToTop, Origin) => Bottom,
                (_, Center) => self,
            }
        } else {
            self
        }
    }
}

key!(AlignmentKey, "alignment",
    "origin" => Align(Origin),
    "center" => Align(Center),
    "end"    => Align(End),

    "left"   => Left,
    "top"    => Top,
    "right"  => Right,
    "bottom" => Bottom,
);
