use crate::layout::GridNode;

use super::*;

/// `stack`: Stack children along an axis.
///
/// # Positional parameters
/// - Children: variadic, of type `template`.
///
/// # Named parameters
/// - Column widths: `columns`, of type `Array<GridUnit>`.
/// - Row widths: `rows`, of type `Array<GridUnit>`.
/// - Gutter: `gutter-vertical` and `gutter-horizontal` for individual track axis or `gutter` for both, of type `Array<GridUnit>` respectively.
/// - Stacking direction: `dir`, of type `direction`.
///
/// # Return value
/// A template that arranges its children along the specified grid cells.
///
/// # Relevant types and constants
/// - Type `direction`
///   - `ltr`
///   - `rtl`
///   - `ttb`
///   - `btt`
pub fn grid(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let cols = args.eat_named::<GridUnits>(ctx, "columns").unwrap_or_default();
    let rows = args.eat_named::<GridUnits>(ctx, "rows").unwrap_or_default();

    let gutter = args.eat_named(ctx, "gutter");
    let gutter_vertical = args
        .eat_named::<GridUnits>(ctx, "gutter-col")
        .or_else(|| gutter.clone())
        .unwrap_or_default();
    let gutter_horizontal = args
        .eat_named::<GridUnits>(ctx, "gutter-row")
        .or(gutter)
        .unwrap_or_default();

    let dir = args.eat_named(ctx, "dir");
    let children = args.eat_all::<TemplateValue>(ctx);

    Value::template("grid", move |ctx| {
        let children =
            children.iter().map(|child| ctx.exec_template_stack(child).into()).collect();
        ctx.push(GridNode {
            dir: dir.unwrap_or_else(|| ctx.state.lang.dir),
            children,
            gutter: Gen::new(gutter_vertical.clone(), gutter_horizontal.clone()),
            tracks: Gen::new(cols.clone(), rows.clone()),
        })
    })
}

/// A list of [`GridUnit`]s.
#[derive(Default, Debug, Clone, PartialEq, Hash)]
pub struct GridUnits(pub Vec<TrackSizing>);

impl GridUnits {
    pub fn get(&self, index: usize) -> TrackSizing {
        if self.0.is_empty() {
            TrackSizing::Auto
        } else {
            *self.0.get(index).unwrap_or(self.0.last().unwrap())
        }
    }
}

value! {
    GridUnits: "array of fractional values, lengths, and the `auto` keyword",
    Value::TrackSizing(value) => Self(vec![value]),
    Value::Array(values) => Self(values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .collect()
    ),
}
