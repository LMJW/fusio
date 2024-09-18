use std::{future::Future, pin::Pin};

use bytes::Bytes;

use crate::{Error, IoBuf, Read, Write};

#[cfg(not(feature = "no-send"))]
pub trait MaybeSend: Send {}

#[cfg(feature = "no-send")]
pub trait MaybeSend {}

#[cfg(not(feature = "no-send"))]
impl<T: Send> MaybeSend for T {}
#[cfg(feature = "no-send")]
impl<T> MaybeSend for T {}

pub trait MaybeSendFuture: Future + MaybeSend {}

impl<F: Future + MaybeSend> MaybeSendFuture for F {}

pub trait DynWrite {
    fn write(
        &mut self,
        buf: Bytes,
    ) -> Pin<Box<dyn MaybeSendFuture<Output = (Result<usize, Error>, Bytes)> + '_>>;

    fn sync_data(&mut self) -> Pin<Box<dyn MaybeSendFuture<Output = Result<(), Error>> + '_>>;

    fn sync_all(&mut self) -> Pin<Box<dyn MaybeSendFuture<Output = Result<(), Error>> + '_>>;

    fn close(&mut self) -> Pin<Box<dyn MaybeSendFuture<Output = Result<(), Error>> + '_>>;
}

impl<W: Write> DynWrite for W {
    fn write(
        &mut self,
        buf: Bytes,
    ) -> Pin<Box<dyn MaybeSendFuture<Output = (Result<usize, Error>, Bytes)> + '_>> {
        Box::pin(W::write(self, buf))
    }

    fn sync_data(&mut self) -> Pin<Box<dyn MaybeSendFuture<Output = Result<(), Error>> + '_>> {
        Box::pin(W::sync_data(self))
    }

    fn sync_all(&mut self) -> Pin<Box<dyn MaybeSendFuture<Output = Result<(), Error>> + '_>> {
        Box::pin(W::sync_all(self))
    }

    fn close(&mut self) -> Pin<Box<dyn MaybeSendFuture<Output = Result<(), Error>> + '_>> {
        Box::pin(W::close(self))
    }
}

pub trait DynRead {
    fn read(
        &mut self,
        pos: u64,
        len: Option<u64>,
    ) -> Pin<Box<dyn MaybeSendFuture<Output = Result<Bytes, Error>> + '_>>;
}

impl<R> DynRead for R
where
    R: Read,
{
    fn read(
        &mut self,
        pos: u64,
        len: Option<u64>,
    ) -> Pin<Box<dyn MaybeSendFuture<Output = Result<Bytes, Error>> + '_>> {
        Box::pin(async move {
            let buf = R::read(self, pos, len).await?;
            Ok(buf.as_bytes())
        })
    }
}

#[cfg(feature = "fs")]
pub use fs::*;

#[cfg(feature = "fs")]
pub mod fs {
    use std::pin::Pin;

    use futures_core::Stream;

    use super::MaybeSendFuture;
    use crate::{
        fs::{FileMeta, Fs},
        path::Path,
        DynRead, DynWrite, Error,
    };

    pub trait DynFile: DynRead + DynWrite {}

    impl<F> DynFile for F where F: DynRead + DynWrite {}

    pub trait DynFs {
        fn open<'s, 'path: 's>(
            &'s self,
            path: &'path Path,
        ) -> Pin<Box<dyn MaybeSendFuture<Output = Result<Box<dyn DynFile + 's>, Error>> + 's>>;

        fn list<'s, 'path: 's>(
            &'s self,
            path: &'path Path,
        ) -> Pin<
            Box<
                dyn MaybeSendFuture<
                        Output = Result<
                            Pin<Box<dyn Stream<Item = Result<FileMeta, Error>> + 's>>,
                            Error,
                        >,
                    > + 's,
            >,
        >;

        fn remove<'s, 'path: 's>(
            &'s self,
            path: &'path Path,
        ) -> Pin<Box<dyn MaybeSendFuture<Output = Result<(), Error>> + 's>>;
    }

    impl<F: Fs> DynFs for F {
        fn open<'s, 'path: 's>(
            &'s self,
            path: &'path Path,
        ) -> Pin<Box<dyn MaybeSendFuture<Output = Result<Box<dyn DynFile + 's>, Error>> + 's>>
        {
            Box::pin(async move {
                let file = F::open(self, path).await?;
                Ok(Box::new(file) as Box<dyn DynFile>)
            })
        }

        fn list<'s, 'path: 's>(
            &'s self,
            path: &'path Path,
        ) -> Pin<
            Box<
                dyn MaybeSendFuture<
                        Output = Result<
                            Pin<Box<dyn Stream<Item = Result<FileMeta, Error>> + 's>>,
                            Error,
                        >,
                    > + 's,
            >,
        > {
            Box::pin(async move {
                let stream = F::list(self, path).await?;
                Ok(Box::pin(stream) as Pin<Box<dyn Stream<Item = Result<FileMeta, Error>>>>)
            })
        }

        fn remove<'s, 'path: 's>(
            &'s self,
            path: &'path Path,
        ) -> Pin<Box<dyn MaybeSendFuture<Output = Result<(), Error>> + 's>> {
            Box::pin(F::remove(self, path))
        }
    }
}