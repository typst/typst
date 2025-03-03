use typst_library::diag::{warning, At, SourceResult};
use typst_library::foundations::{
    Element, Fields, Func, Recipe, Selector, ShowableSelector, Styles, Transformation,
};
use typst_library::layout::BlockElem;
use typst_library::model::ParElem;
use typst_syntax::ast::{self, AstNode};

use crate::{Eval, Vm};

impl Eval for ast::SetRule<'_> {
    type Output = Styles;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        if let Some(condition) = self.condition() {
            if !condition.eval(vm)?.cast::<bool>().at(condition.span())? {
                return Ok(Styles::new());
            }
        }

        let target = self.target();
        let target = target
            .eval(vm)?
            .cast::<Func>()
            .and_then(|func| {
                func.element().ok_or_else(|| {
                    "only element functions can be used in set rules".into()
                })
            })
            .at(target.span())?;
        let args = self.args().eval(vm)?.spanned(self.span());
        Ok(target.set(&mut vm.engine, args)?.spanned(self.span()).liftable())
    }
}

impl Eval for ast::ShowRule<'_> {
    type Output = Recipe;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let selector = self
            .selector()
            .map(|sel| sel.eval(vm)?.cast::<ShowableSelector>().at(sel.span()))
            .transpose()?
            .map(|selector| selector.0);

        let transform = self.transform();
        let transform = match transform {
            ast::Expr::SetRule(set) => Transformation::Style(set.eval(vm)?),
            expr => expr.eval(vm)?.cast::<Transformation>().at(transform.span())?,
        };

        let recipe = Recipe::new(selector, transform, self.span());
        check_show_par_set_block(vm, &recipe);

        Ok(recipe)
    }
}

/// Migration hint for `show par: set block(spacing: ..)`.
fn check_show_par_set_block(vm: &mut Vm, recipe: &Recipe) {
    if_chain::if_chain! {
        if let Some(Selector::Elem(elem, _)) = recipe.selector();
        if *elem == Element::of::<ParElem>();
        if let Transformation::Style(styles) = recipe.transform();
        if styles.has::<BlockElem>(<BlockElem as Fields>::Enum::Above as _) ||
           styles.has::<BlockElem>(<BlockElem as Fields>::Enum::Below as _);
        then {
            vm.engine.sink.warn(warning!(
                recipe.span(),
                "`show par: set block(spacing: ..)` has no effect anymore";
                hint: "write `set par(spacing: ..)` instead";
                hint: "this is specific to paragraphs as they are not considered blocks anymore"
            ))
        }
    }
}
