use std::future::{Future, IntoFuture};
use std::pin::Pin;

pub struct Task<A>(Pin<Box<dyn Future<Output = Result<A, String>> + Send>>);

impl<A: Send + 'static> Task<A> {
    pub fn new(fut: impl Future<Output = Result<A, String>> + Send + 'static) -> Self {
        Task(Box::pin(fut))
    }

    pub fn run(self) -> Pin<Box<dyn Future<Output = Result<A, String>> + Send>> {
        self.0
    }
}

impl<A: Send + 'static> IntoFuture for Task<A> {
    type Output = Result<A, String>;
    type IntoFuture = Pin<Box<dyn Future<Output = Result<A, String>> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        self.0
    }
}

impl<A: Send + 'static> Task<A> {

    pub fn map<B, F>(self, f: F) -> Task<B>
    where
        F: FnOnce(A) -> B + Send + 'static,
        B: Send + 'static,
    {
        Task::new(async move { self.run().await.map(f) })
    }

    pub fn and_then<B, F>(self, f: F) -> Task<B>
    where
        F: FnOnce(A) -> Task<B> + Send + 'static,
        B: Send + 'static,
    {
        Task::new(async move {
            let val = self.run().await?;
            f(val).run().await
        })
    }

    pub fn from_value(val: A) -> Self {
        Task::new(async move { Ok(val) })
    }
}

impl<A: Send + 'static> From<Result<A, String>> for Task<A> {
    fn from(result: Result<A, String>) -> Self {
        Task::new(async move { result })
    }
}
