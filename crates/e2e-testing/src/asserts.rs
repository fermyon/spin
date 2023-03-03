use std::fmt;

#[macro_export]
macro_rules! ensure {
    ($cond:expr $(,)?) => {{
        use anyhow::ensure;
        ensure!($cond, None);
    }};
    ($cond:expr, $($arg:tt)+) => {{
        use anyhow::ensure;
        ensure!($cond, None);
    }};
}

#[macro_export]
macro_rules! ensure_eq {
    ($left:expr, $right:expr $(,)?) => {{
        match (&$left, &$right) {
            (left_val, right_val) => {
                use anyhow::ensure;
                use $crate::asserts::error_msg;
                ensure!(*left_val == *right_val, error_msg("==", &*left_val, &*right_val, None));
            }
        }
    }};
    ($left:expr, $right:expr, $($arg:tt)+) => {{
        match (&$left, &$right) {
            (left_val, right_val) => {
                use anyhow::ensure;
                use $crate::asserts::error_msg;
                ensure!(*left_val == *right_val, error_msg("==", &*left_val, &*right_val, Some(format_args!($($arg)+))))
            }
        }
    }};
}

#[macro_export]
macro_rules! ensure_ne {
    ($left:expr, $right:expr $(,)?) => {{
        match (&$left, &$right) {
            (left_val, right_val) => {
                use anyhow::ensure;
                use $crate::asserts::error_msg;
                ensure!(*left_val != *right_val, error_msg("!=", &*left_val, &*right_val, None));
            }
        }
    }};
    ($left:expr, $right:expr, $($arg:tt)+) => {{
        match (&$left, &$right) {
            (left_val, right_val) => {
                use anyhow::ensure;
                use $crate::asserts::error_msg;
                ensure!(*left_val != *right_val, error_msg("!=", &*left_val, &*right_val, Some(format_args!($($arg)+))))
            }
        }
    }};
}

pub fn error_msg<T, U>(op: &str, left: &T, right: &U, args: Option<fmt::Arguments<'_>>) -> String
where
    T: fmt::Debug + ?Sized,
    U: fmt::Debug + ?Sized,
{
    match args {
        Some(args) => format!(
            r#"assertion failed: `(left {} right)`
  left: `{:?}`,
 right: `{:?}`: {}"#,
            op, left, right, args
        ),
        None => format!(
            r#"assertion failed: `(left {} right)`
  left: `{:?}`,
 right: `{:?}`"#,
            op, left, right,
        ),
    }
}
