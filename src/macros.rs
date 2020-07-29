#![allow(unused)]

/// Unwrap the result if it is `Ok(T)` or evaluate `$or` if it is `Err(_)`.
/// This fits use cases the `?`-operator does not cover, like:
/// ```
/// try_or!(result, continue);
/// ```
macro_rules! try_or {
    ($result:expr, $or:expr $(,)?) => {
        match $result {
            Ok(v) => v,
            Err(_) => { $or }
        }
    };
}

/// Unwrap the option if it is `Some(T)` or evaluate `$or` if it is `None`.
macro_rules! try_opt_or {
    ($option:expr, $or:expr $(,)?) => {
        match $option {
            Some(v) => v,
            None => { $or }
        }
    };
}
