/// Unwrap the option if it is `Some(T)` or evaluate `$or` if it is `None`.
#[allow(unused)]
macro_rules! try_or {
    ($option:expr, $or:expr $(,)?) => {
        match $option {
            Some(v) => v,
            None => $or,
        }
    };
}
