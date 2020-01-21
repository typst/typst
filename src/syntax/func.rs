use super::*;


#[derive(Debug, Clone, PartialEq)]
pub struct FuncHeader {
    pub name: Spanned<Ident>,
    pub args: FuncArgs,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncArgs {
    pub pos: Tuple,
    pub key: Object,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Pos(Spanned<Expr>),
    Key(Pair),
}

impl Arg {
    /// The span or the value or combined span of key and value.
    pub fn span(&self) -> Span {
        match self {
            Arg::Pos(item) => item.span,
            Arg::Key(Pair { key, value }) => Span::merge(key.span, value.span),
        }
    }
}

impl FuncArgs {
    pub fn new() -> FuncArgs {
        FuncArgs {
            pos: Tuple::new(),
            key: Object::new(),
        }
    }

    /// Add an argument.
    pub fn add(&mut self, arg: Arg) {
        match arg {
            Arg::Pos(item) => self.add_pos(item),
            Arg::Key(pair) => self.add_key_pair(pair),
        }
    }

    /// Add a positional argument.
    pub fn add_pos(&mut self, item: Spanned<Expr>) {
        self.pos.add(item);
    }

    /// Add a keyword argument.
    pub fn add_key(&mut self, key: Spanned<Ident>, value: Spanned<Expr>) {
        self.key.add(key, value);
    }

    /// Add a keyword argument from an existing pair.
    pub fn add_key_pair(&mut self, pair: Pair) {
        self.key.add_pair(pair);
    }

    // /// Force-extract the first positional argument.
    // pub fn get_pos<E: ExpressionKind>(&mut self) -> ParseResult<E> {
    //     expect(self.get_pos_opt())
    // }

    // /// Extract the first positional argument.
    // pub fn get_pos_opt<E: ExpressionKind>(&mut self) -> ParseResult<Option<E>> {
    //     Ok(if !self.positional.items.is_empty() {
    //         let spanned = self.positional.items.remove(0);
    //         Some(E::from_expr(spanned)?)
    //     } else {
    //         None
    //     })
    // }

    // /// Force-extract a keyword argument.
    // pub fn get_key<E: ExpressionKind>(&mut self, name: &str) -> ParseResult<E> {
    //     expect(self.get_key_opt(name))
    // }

    // /// Extract a keyword argument.
    // pub fn get_key_opt<E: ExpressionKind>(&mut self, name: &str) -> ParseResult<Option<E>> {
    //     self.keyword.pairs.iter()
    //         .position(|p| p.key.v.0 == name)
    //         .map(|index| {
    //             let value = self.keyword.pairs.swap_remove(index).value;
    //             E::from_expr(value)
    //         })
    //         .transpose()
    // }

    // /// Iterator over positional arguments.
    // pub fn iter_pos(&mut self) -> std::vec::IntoIter<Spanned<Expr>> {
    //     let tuple = std::mem::replace(&mut self.positional, Tuple::new());
    //     tuple.items.into_iter()
    // }

    // /// Iterator over all keyword arguments.
    // pub fn iter_keys(&mut self) -> std::vec::IntoIter<Pair> {
    //     let object = std::mem::replace(&mut self.keyword, Object::new());
    //     object.pairs.into_iter()
    // }

    // /// Clear the argument lists.
    // pub fn clear(&mut self) {
    //     self.positional.items.clear();
    //     self.keyword.pairs.clear();
    // }

    // /// Whether both the positional and keyword argument lists are empty.
    // pub fn is_empty(&self) -> bool {
    //     self.positional.items.is_empty() && self.keyword.pairs.is_empty()
    // }
}

// /// Extract the option expression kind from the option or return an error.
// fn expect<E: ExpressionKind>(opt: ParseResult<Option<E>>) -> ParseResult<E> {
//     match opt {
//         Ok(Some(spanned)) => Ok(spanned),
//         Ok(None) => error!("expected {}", E::NAME),
//         Err(e) => Err(e),
//     }
// }
