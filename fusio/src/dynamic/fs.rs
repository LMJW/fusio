use std::pin::Pin;

use futures_core::Stream;

use super::{DynSeek, MaybeSendFuture};
use crate::{
    fs::{FileMeta, Fs, OpenOptions},
    path::Path,
    DynRead, DynWrite, Error, IoBuf, MaybeSend, MaybeSync, Read, Seek, Write,
};

pub trait DynFile: DynRead + DynSeek + DynWrite + 'static {}

impl<F> DynFile for F where F: DynRead + DynSeek + DynWrite + 'static {}

impl<'seek> Seek for Box<dyn DynFile + 'seek> {
    async fn seek(&mut self, pos: u64) -> Result<(), Error> {
        DynSeek::seek(self.as_mut(), pos).await
    }
}

impl<'read> Read for Box<dyn DynFile + 'read> {
    async fn read(&mut self, len: Option<u64>) -> Result<impl IoBuf, Error> {
        DynRead::read(self.as_mut(), len).await
    }

    async fn size(&self) -> Result<u64, Error> {
        DynRead::size(self.as_ref()).await
    }
}

impl<'write> Write for Box<dyn DynFile + 'write> {
    async fn write<B: IoBuf>(&mut self, buf: B) -> (Result<usize, Error>, B) {
        let (result, _) = DynWrite::write(self.as_mut(), buf.as_bytes()).await;
        (result, buf)
    }

    async fn sync_data(&self) -> Result<(), Error> {
        DynWrite::sync_data(self.as_ref()).await
    }

    async fn sync_all(&self) -> Result<(), Error> {
        DynWrite::sync_all(self.as_ref()).await
    }

    async fn close(&mut self) -> Result<(), Error> {
        DynWrite::close(self.as_mut()).await
    }
}

pub trait DynFs: MaybeSend + MaybeSync {
    fn open<'s, 'path: 's>(
        &'s self,
        path: &'path Path,
    ) -> Pin<Box<dyn MaybeSendFuture<Output = Result<Box<dyn DynFile>, Error>> + 's>> {
        self.open_options(path, OpenOptions::default())
    }

    fn open_options<'s, 'path: 's>(
        &'s self,
        path: &'path Path,
        options: OpenOptions,
    ) -> Pin<Box<dyn MaybeSendFuture<Output = Result<Box<dyn DynFile>, Error>> + 's>>;

    fn create_dir<'s, 'path: 's>(
        &'s self,
        path: &'path Path,
    ) -> Pin<Box<dyn MaybeSendFuture<Output = Result<(), Error>> + 's>>;

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
    fn open_options<'s, 'path: 's>(
        &'s self,
        path: &'path Path,
        options: OpenOptions,
    ) -> Pin<Box<dyn MaybeSendFuture<Output = Result<Box<dyn DynFile>, Error>> + 's>> {
        Box::pin(async move {
            let file = F::open_options(self, path, options).await?;
            Ok(Box::new(file) as Box<dyn DynFile>)
        })
    }

    fn create_dir<'s, 'path: 's>(
        &'s self,
        path: &'path Path,
    ) -> Pin<Box<dyn MaybeSendFuture<Output = Result<(), Error>> + 's>> {
        Box::pin(F::create_dir(path))
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
