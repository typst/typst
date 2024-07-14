mod collect;
mod finalize;
mod line;
mod linebreak;
mod prepare;
mod shaping;

use comemo::{Track, Tracked, TrackedMut};

use self::collect::{collect, Item, Segment, SpanMapper};
use self::finalize::finalize;
use self::line::{commit, line, Line};
use self::linebreak::{is_default_ignorable, linebreak, Breakpoint};
use self::prepare::{prepare, Preparation};
use self::shaping::{
    cjk_punct_style, is_of_cj_script, shape_range, ShapedGlyph, ShapedText,
    BEGIN_PUNCT_PAT, END_PUNCT_PAT,
};
use crate::diag::SourceResult;
use crate::engine::{Engine, Route, Sink, Traced};
use crate::foundations::StyleChain;
use crate::introspection::{Introspector, Locator, LocatorLink};
use crate::layout::{Fragment, Size};
use crate::model::ParElem;
use crate::realize::StyleVec;
use crate::World;

/// Range of a substring of text.
type Range = std::ops::Range<usize>;

/// Layouts content inline.
pub(crate) fn layout_inline(
    children: &StyleVec,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    consecutive: bool,
    region: Size,
    expand: bool,
) -> SourceResult<Fragment> {
    #[comemo::memoize]
    #[allow(clippy::too_many_arguments)]
    fn cached(
        children: &StyleVec,
        world: Tracked<dyn World + '_>,
        introspector: Tracked<Introspector>,
        traced: Tracked<Traced>,
        sink: TrackedMut<Sink>,
        route: Tracked<Route>,
        locator: Tracked<Locator>,
        styles: StyleChain,
        consecutive: bool,
        region: Size,
        expand: bool,
    ) -> SourceResult<Fragment> {
        let link = LocatorLink::new(locator);
        let locator = Locator::link(&link);
        let mut engine = Engine {
            world,
            introspector,
            traced,
            sink,
            route: Route::extend(route),
        };

        // Collect all text into one string for BiDi analysis.
        let (text, segments, spans) =
            collect(children, &mut engine, locator, &styles, region, consecutive)?;

        // Perform BiDi analysis and then prepares paragraph layout.
        let p = prepare(&mut engine, children, &text, segments, spans, styles)?;

        // Break the paragraph into lines.
        let lines = linebreak(&engine, &p, region.x - p.hang);

        // Turn the selected lines into frames.
        finalize(&mut engine, &p, &lines, styles, region, expand)
    }

    cached(
        children,
        engine.world,
        engine.introspector,
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        locator.track(),
        styles,
        consecutive,
        region,
        expand,
    )
}
