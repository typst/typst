use crate::layout::{GridNode, TrackSizing, Tracks};

use super::*;

/// `grid`: Arrange children into a grid.
///
/// # Positional parameters
/// - Children: variadic, of type `template`.
///
/// # Named parameters
/// - Column sizing: `columns`, of type `tracks`.
/// - Row sizing: `rows`, of type `tracks`.
/// - Column direction: `column-dir`, of type `direction`.
/// - Gutter: `gutter`, shorthand for equal column and row gutter, of type `tracks`.
/// - Gutter for rows: `gutter-rows`, of type `tracks`.
/// - Gutter for columns: `gutter-columns`, of type `tracks`.
///
/// # Return value
/// A template that arranges its children along the specified grid cells.
///
/// # Relevant types and constants
/// - Type `tracks`
///   - coerces from `array` of `track-sizing`
/// - Type `track-sizing`
///   - `auto`
//    - coerces from `length`
//    - coerces from `relative`
//    - coerces from `linear`
//    - coerces from `fractional`
/// - Type `direction`
///   - `ltr`
///   - `rtl`
///   - `ttb`
///   - `btt`
pub fn grid(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let columns = args.eat_named::<Tracks>(ctx, "columns").unwrap_or_default();
    let rows = args.eat_named::<Tracks>(ctx, "rows").unwrap_or_default();
    let column_dir = args.eat_named(ctx, "column-dir");
    let gutter = args.eat_named::<Tracks>(ctx, "gutter").unwrap_or_default();
    let gutter_columns = args.eat_named::<Tracks>(ctx, "gutter-columns");
    let gutter_rows = args.eat_named::<Tracks>(ctx, "gutter-rows");
    let children = args.eat_all::<TemplateValue>(ctx);

    Value::template("grid", move |ctx| {
        let children = children
            .iter()
            .map(|child| ctx.exec_template_stack(child).into())
            .collect();

        ctx.push_into_stack(GridNode {
            column_dir: column_dir.unwrap_or(ctx.state.lang.dir),
            children,
            tracks: Gen::new(columns.clone(), rows.clone()),
            gutter: Gen::new(
                gutter_columns.as_ref().unwrap_or(&gutter).clone(),
                gutter_rows.as_ref().unwrap_or(&gutter).clone(),
            ),
        })
    })
}

value! {
    Tracks: "array of `auto`s, linears, and fractionals",
    Value::Int(count) => Self(vec![TrackSizing::Auto; count.max(0) as usize]),
    Value::Array(values) => Self(values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .collect()
    ),
}

value! {
    TrackSizing: "`auto`, linear, or fractional",
    Value::Auto => TrackSizing::Auto,
    Value::Length(v) => TrackSizing::Linear(v.into()),
    Value::Relative(v) => TrackSizing::Linear(v.into()),
    Value::Linear(v) => TrackSizing::Linear(v),
    Value::Fractional(v) => TrackSizing::Fractional(v),
}
