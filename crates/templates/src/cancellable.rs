pub(crate) enum Cancellable<T, E> {
    Cancelled,
    Err(E),
    Ok(T),
}

impl<T, E> Cancellable<T, E> {
    pub(crate) fn err(self) -> Result<(), E> {
        match self {
            Self::Ok(_) => Ok(()),
            Self::Cancelled => Ok(()),
            Self::Err(e) => Err(e),
        }
    }

    pub(crate) fn and_then<U>(self, f: impl Fn(T) -> Result<U, E>) -> Cancellable<U, E> {
        match self {
            Self::Ok(value) => match f(value) {
                Ok(r) => Cancellable::Ok(r),
                Err(e) => Cancellable::Err(e),
            },
            Self::Cancelled => Cancellable::Cancelled,
            Self::Err(e) => Cancellable::Err(e),
        }
    }

    pub(crate) async fn and_then_async<U, Fut: std::future::Future<Output = Result<U, E>>>(
        self,
        f: impl Fn(T) -> Fut,
    ) -> Cancellable<U, E> {
        match self {
            Self::Ok(value) => match f(value).await {
                Ok(r) => Cancellable::Ok(r),
                Err(e) => Cancellable::Err(e),
            },
            Self::Cancelled => Cancellable::Cancelled,
            Self::Err(e) => Cancellable::Err(e),
        }
    }
}

impl<T, E> From<Result<Option<T>, E>> for Cancellable<T, E> {
    fn from(ro: Result<Option<T>, E>) -> Self {
        match ro {
            Ok(Some(t)) => Self::Ok(t),
            Ok(None) => Self::Cancelled,
            Err(e) => Self::Err(e),
        }
    }
}
