use typst_library::diag::{At, SourceResult, warning};
use typst_library::foundations::{
    Element, Func, Recipe, Selector, ShowableSelector, Styles, Transformation,
};
use typst_library::layout::{BlockElem, PageElem};
use typst_library::model::ParElem;
use typst_syntax::ast::{self, AstNode};

use crate::{Eval, Vm, hint_if_shadowed_std};

impl Eval for ast::SetRule<'_> {
    type Output = Styles;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        if let Some(condition) = self.condition()
            && !condition.eval(vm)?.cast::<bool>().at(condition.span())?
        {
            return Ok(Styles::new());
        }

        let target_expr = self.target();
        let target = target_expr
            .eval(vm)?
            .cast::<Func>()
            .map_err(|err| hint_if_shadowed_std(vm, &target_expr, err))
            .and_then(|func| {
                func.to_element().ok_or_else(|| {
                    "only element functions can be used in set rules".into()
                })
            })
            .at(target_expr.span())?;
        let args = self.args().eval(vm)?.spanned(self.span());
        Ok(target.set(&mut vm.engine, args)?.spanned(self.span()).liftable())
    }
}

impl Eval for ast::ShowRule<'_> {
    type Output = Recipe;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let selector = self
            .selector()
            .map(|sel| {
                sel.eval(vm)?
                    .cast::<ShowableSelector>()
                    .map_err(|err| hint_if_shadowed_std(vm, &sel, err))
                    .at(sel.span())
            })
            .transpose()?
            .map(|selector| selector.0);

        let transform = self.transform();
        let transform = match transform {
            ast::Expr::SetRule(set) => Transformation::Style(set.eval(vm)?),
            expr => expr.eval(vm)?.cast::<Transformation>().at(transform.span())?,
        };

        let recipe = Recipe::new(selector, transform, self.span());
        check_show_page_rule(vm, &recipe);
        check_show_par_set_block(vm, &recipe);

        Ok(recipe)
    }
}

/// Warns that `show page` rules currently have no effect.
fn check_show_page_rule(vm: &mut Vm, recipe: &Recipe) {
    if let Some(Selector::Elem(elem, _)) = recipe.selector()
        && *elem == Element::of::<PageElem>()
    {
        vm.engine.sink.warn(warning!(
            recipe.span(),
            "`show page` is not supported and has no effect";
            hint: "customize pages with `set page(..)` instead";
        ));
    }
}

/// Migration hint for `show par: set block(spacing: ..)`.
fn check_show_par_set_block(vm: &mut Vm, recipe: &Recipe) {
    if let Some(Selector::Elem(elem, _)) = recipe.selector()
        && *elem == Element::of::<ParElem>()
        && let Transformation::Style(styles) = recipe.transform()
        && (styles.has(BlockElem::above) || styles.has(BlockElem::below))
    {
        vm.engine.sink.warn(warning!(
            recipe.span(),
            "`show par: set block(spacing: ..)` has no effect anymore";
            hint: "write `set par(spacing: ..)` instead";
            hint: "this is specific to paragraphs as they are not considered blocks \
                   anymore";
        ))
    }
}
