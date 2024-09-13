//! Uwu

/// Uwu
#[allow_internal_unstable(builtin_syntax)]
#[unstable(feature = "unsafe_binders", issue = "none")]
pub macro unwrap_unsafe_binder {
    ($expr:expr) => {
        builtin # unwrap_unsafe_binder ( $expr )
    },
    ($expr:expr ; $ty:ty) => {
        builtin # unwrap_unsafe_binder ( $expr, $ty )
    },
}

/// Uwu
#[allow_internal_unstable(builtin_syntax)]
#[unstable(feature = "unsafe_binders", issue = "none")]
pub macro wrap_unsafe_binder {
    ($expr:expr) => {
        builtin # wrap_unsafe_binder ( $expr )
    },
    ($expr:expr ; $ty:ty) => {
        builtin # wrap_unsafe_binder ( $expr, $ty )
    },
}
