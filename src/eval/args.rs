//! Simplifies argument parsing.

use std::mem;

use super::*;

/// A wrapper around a dictionary value that simplifies argument parsing in
/// functions.
pub struct Args(pub Spanned<ValueDict>);

impl Args {
    /// Retrieve and remove the argument associated with the given key if there
    /// is any.
    ///
    /// Generates an error if the key exists, but the value can't be converted
    /// into the type `T`.
    pub fn get<'a, K, T>(&mut self, ctx: &mut LayoutContext, key: K) -> Option<T>
    where
        K: Into<RefKey<'a>>,
        T: Convert,
    {
        self.0.v.remove(key).and_then(|entry| {
            let span = entry.value.span;
            let (t, diag) = T::convert(entry.value);
            if let Some(diag) = diag {
                ctx.f.diags.push(diag.span_with(span))
            }
            t.ok()
        })
    }

    /// Retrieve and remove the first matching positional argument.
    pub fn find<T>(&mut self) -> Option<T>
    where
        T: Convert,
    {
        for (&key, entry) in self.0.v.nums_mut() {
            let span = entry.value.span;
            match T::convert(mem::take(&mut entry.value)).0 {
                Ok(t) => {
                    self.0.v.remove(key);
                    return Some(t);
                }
                Err(v) => entry.value = v.span_with(span),
            }
        }
        None
    }

    /// Retrieve and remove all matching positional arguments.
    pub fn find_all<T>(&mut self) -> impl Iterator<Item = T> + '_
    where
        T: Convert,
    {
        let mut skip = 0;
        std::iter::from_fn(move || {
            for (&key, entry) in self.0.v.nums_mut().skip(skip) {
                let span = entry.value.span;
                match T::convert(mem::take(&mut entry.value)).0 {
                    Ok(t) => {
                        self.0.v.remove(key);
                        return Some(t);
                    }
                    Err(v) => entry.value = v.span_with(span),
                }
                skip += 1;
            }
            None
        })
    }

    /// Retrieve and remove all matching keyword arguments.
    pub fn find_all_str<T>(&mut self) -> impl Iterator<Item = (String, T)> + '_
    where
        T: Convert,
    {
        let mut skip = 0;
        std::iter::from_fn(move || {
            for (key, entry) in self.0.v.strs_mut().skip(skip) {
                let span = entry.value.span;
                match T::convert(mem::take(&mut entry.value)).0 {
                    Ok(t) => {
                        let key = key.clone();
                        self.0.v.remove(&key);
                        return Some((key, t));
                    }
                    Err(v) => entry.value = v.span_with(span),
                }
                skip += 1;
            }

            None
        })
    }

    /// Generated _unexpected argument_ errors for all remaining entries.
    pub fn done(&self, ctx: &mut LayoutContext) {
        for entry in self.0.v.values() {
            let span = entry.key_span.join(entry.value.span);
            error!(@ctx.f, span, "unexpected argument");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(value: Value) -> SpannedEntry<Value> {
        SpannedEntry::value(Spanned::zero(value))
    }

    #[test]
    fn test_args_find() {
        let mut args = Args(Spanned::zero(Dict::new()));
        args.0.v.insert(1, entry(Value::Bool(false)));
        args.0.v.insert(2, entry(Value::Str("hi".to_string())));
        assert_eq!(args.find::<String>(), Some("hi".to_string()));
        assert_eq!(args.0.v.len(), 1);
        assert_eq!(args.find::<bool>(), Some(false));
        assert!(args.0.v.is_empty());
    }

    #[test]
    fn test_args_find_all() {
        let mut args = Args(Spanned::zero(Dict::new()));
        args.0.v.insert(1, entry(Value::Bool(false)));
        args.0.v.insert(3, entry(Value::Float(0.0)));
        args.0.v.insert(7, entry(Value::Bool(true)));
        assert_eq!(args.find_all::<bool>().collect::<Vec<_>>(), [false, true]);
        assert_eq!(args.0.v.len(), 1);
        assert_eq!(args.0.v[3].value.v, Value::Float(0.0));
    }
}
