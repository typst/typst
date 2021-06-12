use crate::layout::{GridNode, TrackSizing};

use super::*;

/// `grid`: Arrange children into a grid.
///
/// # Positional parameters
/// - Children: variadic, of type `template`.
///
/// # Named parameters
/// - Column sizing: `columns`, of type `tracks`.
/// - Row sizing: `rows`, of type `tracks`.
/// - Gutter: `gutter`, shorthand for equal gutter everywhere, of type `length`.
/// - Gutter for rows: `gutter-rows`, of type `tracks`.
/// - Gutter for columns: `gutter-columns`, of type `tracks`.
/// - Column direction: `column-dir`, of type `direction`.
/// - Row direction: `row-dir`, of type `direction`.
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
    let gutter = args
        .eat_named::<Linear>(ctx, "gutter")
        .map(|v| vec![TrackSizing::Linear(v)])
        .unwrap_or_default();
    let gutter_columns = args.eat_named::<Tracks>(ctx, "gutter-columns");
    let gutter_rows = args.eat_named::<Tracks>(ctx, "gutter-rows");
    let column_dir = args.eat_named(ctx, "column-dir");
    let row_dir = args.eat_named(ctx, "row-dir");
    let children = args.eat_all::<TemplateValue>(ctx);

    Value::template("grid", move |ctx| {
        let children = children
            .iter()
            .map(|child| ctx.exec_template_stack(child).into())
            .collect();

        let cross_dir = column_dir.unwrap_or(ctx.state.lang.dir);
        let main_dir = row_dir.unwrap_or(cross_dir.axis().other().dir(true));

        ctx.push_into_stack(GridNode {
            dirs: Gen::new(cross_dir, main_dir),
            tracks: Gen::new(columns.clone(), rows.clone()),
            gutter: Gen::new(
                gutter_columns.as_ref().unwrap_or(&gutter).clone(),
                gutter_rows.as_ref().unwrap_or(&gutter).clone(),
            ),
            children,
        })
    })
}

/// Defines size of rows and columns in a grid.
type Tracks = Vec<TrackSizing>;

value! {
    Tracks: "array of `auto`s, linears, and fractionals",
    Value::Int(count) => vec![TrackSizing::Auto; count.max(0) as usize],
    Value::Array(values) => values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .collect(),
}

value! {
    TrackSizing: "`auto`, linear, or fractional",
    Value::Auto => TrackSizing::Auto,
    Value::Length(v) => TrackSizing::Linear(v.into()),
    Value::Relative(v) => TrackSizing::Linear(v.into()),
    Value::Linear(v) => TrackSizing::Linear(v),
    Value::Fractional(v) => TrackSizing::Fractional(v),
}
