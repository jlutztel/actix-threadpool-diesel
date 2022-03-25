use async_trait::async_trait;
use diesel::{
    connection::SimpleConnection,
    dsl::Limit,
    query_dsl::{
        methods::{ExecuteDsl, LimitDsl, LoadQuery},
        RunQueryDsl,
    },
    r2d2::{ConnectionManager, Pool},
    result::Error as DieselError,
    Connection,
};
use std::{error::Error as StdError, fmt};
use tokio::task;

#[derive(Debug)]
pub enum AsyncError<E: fmt::Debug> {
    // Failed to checkout a connection
    Checkout(r2d2::Error),

    // The query failed in some way
    Error(E),

    // The task was cancelled
    Canceled,
}

pub trait OptionalExtension<T, E: fmt::Debug> {
    fn optional(self) -> Result<Option<T>, AsyncError<E>>;
}

impl<T> OptionalExtension<T, DieselError> for Result<T, AsyncError<DieselError>> {
    fn optional(self) -> Result<Option<T>, AsyncError<DieselError>> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(AsyncError::Error(diesel::result::Error::NotFound)) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl<E: fmt::Display + fmt::Debug> fmt::Display for AsyncError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AsyncError::Checkout(ref err) => fmt::Display::fmt(&err, f),
            AsyncError::Error(ref err) => fmt::Display::fmt(&err, f),
            AsyncError::Canceled => write!(f, "task was cancelled"),
        }
    }
}

impl<E: 'static + StdError> StdError for AsyncError<E> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self {
            AsyncError::Checkout(ref err) => Some(err),
            AsyncError::Error(ref err) => Some(err),
            AsyncError::Canceled => None,
        }
    }
}

#[async_trait]
pub trait AsyncSimpleConnection<Conn>
where
    Conn: 'static + SimpleConnection,
{
    async fn batch_execute_async(&self, query: &str) -> Result<(), AsyncError<DieselError>>;
}

#[async_trait]
impl<Conn> AsyncSimpleConnection<Conn> for Pool<ConnectionManager<Conn>>
where
    Conn: 'static + Connection,
{
    #[inline]
    async fn batch_execute_async(&self, query: &str) -> Result<(), AsyncError<DieselError>> {
        let self_ = self.clone();
        let query = query.to_string();
        task::block_in_place(move || {
            let conn = self_.get().map_err(AsyncError::Checkout)?;
            conn.batch_execute(&query).map_err(AsyncError::Error)
        })
    }
}

#[async_trait]
pub trait AsyncConnection<Conn>: AsyncSimpleConnection<Conn>
where
    Conn: 'static + Connection,
{
    async fn run<R, E, Func>(&self, f: Func) -> Result<R, AsyncError<E>>
    where
        R: 'static + Send,
        E: 'static + From<DieselError> + fmt::Debug + Send,
        Func: 'static + FnOnce(&Conn) -> Result<R, E> + Send;

    async fn transaction<R, E, Func>(&self, f: Func) -> Result<R, AsyncError<E>>
    where
        R: 'static + Send,
        E: 'static + From<DieselError> + fmt::Debug + Send,
        Func: 'static + FnOnce(&Conn) -> Result<R, E> + Send;
}

#[async_trait]
impl<Conn> AsyncConnection<Conn> for Pool<ConnectionManager<Conn>>
where
    Conn: 'static + Connection,
{
    #[inline]
    async fn run<R, E, Func>(&self, f: Func) -> Result<R, AsyncError<E>>
    where
        R: 'static + Send,
        E: 'static + From<DieselError> + fmt::Debug + Send,
        Func: 'static + FnOnce(&Conn) -> Result<R, E> + Send,
    {
        let self_ = self.clone();
        task::block_in_place(move || {
            let conn = self_.get().map_err(AsyncError::Checkout)?;
            f(&*conn).map_err(AsyncError::Error)
        })
    }

    #[inline]
    async fn transaction<R, E, Func>(&self, f: Func) -> Result<R, AsyncError<E>>
    where
        R: 'static + Send,
        E: 'static + From<DieselError> + fmt::Debug + Send,
        Func: 'static + FnOnce(&Conn) -> Result<R, E> + Send,
    {
        let self_ = self.clone();
        task::block_in_place(move || {
            let conn = self_.get().map_err(AsyncError::Checkout)?;
            conn.transaction::<R, E, _>(|| f(&*conn))
                .map_err(AsyncError::Error)
        })
    }
}

#[async_trait]
pub trait AsyncRunQueryDsl<Conn, AsyncConn>
where
    Conn: 'static + Connection,
{
    async fn execute_async(self, asc: &AsyncConn) -> Result<usize, AsyncError<DieselError>>
    where
        Self: ExecuteDsl<Conn>;

    async fn load_async<U>(self, asc: &AsyncConn) -> Result<Vec<U>, AsyncError<DieselError>>
    where
        U: 'static + Send,
        Self: LoadQuery<Conn, U>;

    async fn get_result_async<U>(self, asc: &AsyncConn) -> Result<U, AsyncError<DieselError>>
    where
        U: 'static + Send,
        Self: LoadQuery<Conn, U>;

    async fn get_results_async<U>(self, asc: &AsyncConn) -> Result<Vec<U>, AsyncError<DieselError>>
    where
        U: 'static + Send,
        Self: LoadQuery<Conn, U>;

    async fn first_async<U>(self, asc: &AsyncConn) -> Result<U, AsyncError<DieselError>>
    where
        U: 'static + Send,
        Self: LimitDsl,
        Limit<Self>: LoadQuery<Conn, U>;
}

#[async_trait]
impl<T, Conn> AsyncRunQueryDsl<Conn, Pool<ConnectionManager<Conn>>> for T
where
    T: 'static + Send + RunQueryDsl<Conn>,
    Conn: 'static + Connection,
{
    async fn execute_async(
        self,
        asc: &Pool<ConnectionManager<Conn>>,
    ) -> Result<usize, AsyncError<DieselError>>
    where
        Self: ExecuteDsl<Conn>,
    {
        asc.run(|conn| self.execute(&*conn)).await
    }

    async fn load_async<U>(
        self,
        asc: &Pool<ConnectionManager<Conn>>,
    ) -> Result<Vec<U>, AsyncError<DieselError>>
    where
        U: 'static + Send,
        Self: LoadQuery<Conn, U>,
    {
        asc.run(|conn| self.load(&*conn)).await
    }

    async fn get_result_async<U>(
        self,
        asc: &Pool<ConnectionManager<Conn>>,
    ) -> Result<U, AsyncError<DieselError>>
    where
        U: 'static + Send,
        Self: LoadQuery<Conn, U>,
    {
        asc.run(|conn| self.get_result(&*conn)).await
    }

    async fn get_results_async<U>(
        self,
        asc: &Pool<ConnectionManager<Conn>>,
    ) -> Result<Vec<U>, AsyncError<DieselError>>
    where
        U: 'static + Send,
        Self: LoadQuery<Conn, U>,
    {
        asc.run(|conn| self.get_results(&*conn)).await
    }

    async fn first_async<U>(
        self,
        asc: &Pool<ConnectionManager<Conn>>,
    ) -> Result<U, AsyncError<DieselError>>
    where
        U: 'static + Send,
        Self: LimitDsl,
        Limit<Self>: LoadQuery<Conn, U>,
    {
        asc.run(|conn| self.first(&*conn)).await
    }
}
