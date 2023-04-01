use super::*;

/// A bezier path.
#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct PathBuilder<T: 'static + Numeric = Length>(pub Vec<PathBuilderItem<T>>);

/// An item in a bezier path.
#[derive(Clone, Eq, PartialEq, Hash)]
pub enum PathBuilderItem<T: 'static + Numeric> {
    MoveTo(Axes<DeltaAbs<Rel<T>>>),
    LineTo(Axes<DeltaAbs<Rel<T>>>),
    CubicTo(Axes<DeltaAbs<Rel<T>>>, Axes<DeltaAbs<Rel<T>>>, Axes<DeltaAbs<Rel<T>>>),
    ArcTo {
        to: Axes<DeltaAbs<Rel<T>>>,
        radius: Axes<T>,
        x_rotation: Angle,
        large: bool,
        sweep: bool,
    },
    ClosePath,
}

impl<T: Debug + Numeric> Debug for PathBuilderItem<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            PathBuilderItem::MoveTo(p) => {
                write!(f, "draw.moveto(")?;
                write!(
                    f,
                    "{}x: {:?}, ",
                    if p.x.is_delta() { "d" } else { "" },
                    p.x.value()
                )?;
                write!(
                    f,
                    "{}y: {:?})",
                    if p.y.is_delta() { "d" } else { "" },
                    p.y.value()
                )?;
            }
            PathBuilderItem::LineTo(p) => {
                write!(f, "draw.lineto(")?;
                write!(
                    f,
                    "{}x: {:?}, ",
                    if p.x.is_delta() { "d" } else { "" },
                    p.x.value()
                )?;
                write!(
                    f,
                    "{}y: {:?})",
                    if p.y.is_delta() { "d" } else { "" },
                    p.y.value()
                )?;
            }
            PathBuilderItem::CubicTo(p1, p2, p) => {
                write!(f, "draw.cubicto(")?;
                write!(
                    f,
                    "{}x1: {:?}, ",
                    if p1.x.is_delta() { "d" } else { "" },
                    p1.x.value()
                )?;
                write!(
                    f,
                    "{}y1: {:?}, ",
                    if p1.y.is_delta() { "d" } else { "" },
                    p1.y.value()
                )?;
                write!(
                    f,
                    "{}x2: {:?}, ",
                    if p2.x.is_delta() { "d" } else { "" },
                    p2.x.value()
                )?;
                write!(
                    f,
                    "{}y2: {:?}, ",
                    if p2.y.is_delta() { "d" } else { "" },
                    p2.y.value()
                )?;
                write!(
                    f,
                    "{}x: {:?}, ",
                    if p.x.is_delta() { "d" } else { "" },
                    p.x.value()
                )?;
                write!(
                    f,
                    "{}y: {:?})",
                    if p.y.is_delta() { "d" } else { "" },
                    p.y.value()
                )?;
            }
            PathBuilderItem::ArcTo { to, radius, x_rotation, large, sweep } => {
                write!(f, "draw.arcto(")?;
                write!(
                    f,
                    "{}x: {:?}, ",
                    if to.x.is_delta() { "d" } else { "" },
                    to.x.value()
                )?;
                write!(
                    f,
                    "{}y: {:?}, ",
                    if to.y.is_delta() { "d" } else { "" },
                    to.y.value()
                )?;
                write!(f, "radius: ({:?}, {:?}), ", radius.x, radius.y)?;
                write!(f, "x-rotation: {:?}, ", x_rotation)?;
                write!(f, "large: {:?}, ", large)?;
                write!(f, "sweep: {:?})", sweep)?;
            }
            PathBuilderItem::ClosePath => {
                write!(f, "draw.close()")?;
            }
        }
        Ok(())
    }
}

impl<T: Numeric> PathBuilder<T> {
    /// Create an empty path.
    pub const fn new() -> Self {
        Self(vec![])
    }

    /// Push a [`MoveTo`](PathBuilderItem::MoveTo) item.
    pub fn move_to(&mut self, p: Axes<DeltaAbs<Rel<T>>>) {
        self.0.push(PathBuilderItem::MoveTo(p));
    }

    /// Push a [`LineTo`](PathBuilderItem::LineTo) item.
    pub fn line_to(&mut self, p: Axes<DeltaAbs<Rel<T>>>) {
        self.0.push(PathBuilderItem::LineTo(p));
    }

    /// Push a [`CubicTo`](PathBuilderItem::CubicTo) item.
    pub fn cubic_to(
        &mut self,
        p1: Axes<DeltaAbs<Rel<T>>>,
        p2: Axes<DeltaAbs<Rel<T>>>,
        p3: Axes<DeltaAbs<Rel<T>>>,
    ) {
        self.0.push(PathBuilderItem::CubicTo(p1, p2, p3));
    }

    /// Push a [`ArcTo`](PathBuilderItem::ArcTo) item.
    pub fn arc_to(
        &mut self,
        to: Axes<DeltaAbs<Rel<T>>>,
        radius: Axes<T>,
        x_rotation: Angle,
        large: bool,
        sweep: bool,
    ) {
        self.0
            .push(PathBuilderItem::ArcTo { to, radius, x_rotation, large, sweep });
    }

    /// Push a [`ClosePath`](PathBuilderItem::ClosePath) item.
    pub fn close_path(&mut self) {
        self.0.push(PathBuilderItem::ClosePath);
    }

    /// Extend a path with elements from another.
    pub fn extend(&mut self, other: &Self)
    where
        T: Clone,
    {
        self.0.extend(other.0.iter().cloned())
    }
}

impl<T: Debug + Numeric> Debug for PathBuilder<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{{")?;
        for item in &self.0 {
            write!(f, "\n  {:?}", item)?;
        }
        write!(f, "\n}}")?;
        Ok(())
    }
}

impl PathBuilder<Abs> {
    // Compute all relative coordinates of the path
    pub fn to_path(self, base: Size) -> Path {
        struct Builder {
            path: Path,
            endpoint: Point,
            base: Size,
        }

        impl Builder {
            pub fn add_item(&mut self, item: PathItem) {
                if let Some(point) = item.endpoint() {
                    self.endpoint = point;
                }
                self.path.0.push(item);
            }
            pub fn extend(&mut self, item: PathBuilderItem<Abs>) {
                let map_axes = |p: Axes<DeltaAbs<Rel<Abs>>>| Point {
                    x: p.x.map(|l| l.relative_to(self.base.x)).to_abs(self.endpoint.x),
                    y: p.y.map(|l| l.relative_to(self.base.y)).to_abs(self.endpoint.y),
                };
                match item {
                    PathBuilderItem::MoveTo(p) => {
                        self.add_item(PathItem::MoveTo(map_axes(p)))
                    }
                    PathBuilderItem::LineTo(p) => {
                        self.add_item(PathItem::LineTo(map_axes(p)))
                    }
                    PathBuilderItem::CubicTo(p1, p2, p3) => {
                        self.add_item(PathItem::CubicTo(
                            map_axes(p1),
                            map_axes(p2),
                            map_axes(p3),
                        ));
                    }
                    PathBuilderItem::ArcTo { to, radius, x_rotation, large, sweep } => {
                        let arc = kurbo::SvgArc {
                            from: self.endpoint.into(),
                            to: map_axes(to).into(),
                            radii: radius.into(),
                            x_rotation: x_rotation.to_rad(),
                            large_arc: large,
                            sweep,
                        };

                        if arc.is_straight_line() {
                            self.add_item(PathItem::MoveTo(arc.to.into()));
                        } else {
                            // Can only fail if `is_straight_line` returns true.
                            kurbo::Arc::from_svg_arc(&arc).unwrap().to_cubic_beziers(
                                1e-5,
                                |p1, p2, p3| {
                                    self.add_item(PathItem::CubicTo(
                                        p1.into(),
                                        p2.into(),
                                        p3.into(),
                                    ));
                                },
                            );
                        }
                    }
                    PathBuilderItem::ClosePath => self.path.0.push(PathItem::ClosePath),
                }
            }
        }

        let mut builder = Builder {
            path: Path(vec![PathItem::MoveTo(Point::zero())]),
            endpoint: Point::zero(),
            base,
        };

        for item in self.0.into_iter() {
            builder.extend(item);
        }

        builder.path
    }
}

/// A value that is either absolute or a delta of an
/// earlier value.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum DeltaAbs<T> {
    Abs(T),
    Delta(T),
}

impl<T> DeltaAbs<T> {
    // Transform the inner value.
    pub fn map<V, F: FnOnce(T) -> V>(self, f: F) -> DeltaAbs<V> {
        match self {
            DeltaAbs::Abs(v) => DeltaAbs::Abs(f(v)),
            DeltaAbs::Delta(v) => DeltaAbs::Delta(f(v)),
        }
    }

    pub fn is_delta(&self) -> bool {
        match self {
            DeltaAbs::Abs(_) => false,
            DeltaAbs::Delta(_) => true,
        }
    }

    pub fn value(&self) -> &T {
        match self {
            DeltaAbs::Abs(v) => &v,
            DeltaAbs::Delta(v) => &v,
        }
    }
}

impl<T: std::ops::Add<T, Output = T>> DeltaAbs<T> {
    pub fn to_abs(self, endpoint: T) -> T {
        match self {
            DeltaAbs::Abs(v) => v,
            DeltaAbs::Delta(v) => endpoint + v,
        }
    }
}

impl<T: Resolve> Resolve for DeltaAbs<T> {
    type Output = DeltaAbs<<T as Resolve>::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Resolve + Numeric> Resolve for PathBuilderItem<T>
where
    <T as Resolve>::Output: Numeric,
{
    type Output = PathBuilderItem<<T as Resolve>::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        match self {
            PathBuilderItem::MoveTo(p) => PathBuilderItem::MoveTo(p.resolve(styles)),
            PathBuilderItem::LineTo(p) => PathBuilderItem::LineTo(p.resolve(styles)),
            PathBuilderItem::CubicTo(p1, p2, p3) => PathBuilderItem::CubicTo(
                p1.resolve(styles),
                p2.resolve(styles),
                p3.resolve(styles),
            ),
            PathBuilderItem::ArcTo { to, radius, x_rotation, large, sweep } => {
                PathBuilderItem::ArcTo {
                    to: to.resolve(styles),
                    radius: radius.resolve(styles),
                    x_rotation,
                    large,
                    sweep,
                }
            }
            PathBuilderItem::ClosePath => PathBuilderItem::ClosePath,
        }
    }
}

cast_from_value! {
    PathBuilder: "path",
}

impl<T: Resolve + Numeric> Resolve for PathBuilder<T>
where
    <T as Resolve>::Output: Numeric,
{
    type Output = PathBuilder<<T as Resolve>::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        PathBuilder(self.0.into_iter().map(|i| i.resolve(styles)).collect())
    }
}
