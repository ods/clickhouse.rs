use std::{
    pin::Pin,
    task::{Context, Poll},
};

#[cfg(feature = "brotli")]
use async_compression::stream::BrotliDecoder;
#[cfg(feature = "gzip")]
use async_compression::stream::GzipDecoder;
#[cfg(feature = "zlib")]
use async_compression::stream::ZlibDecoder;
use bytes::Bytes;
use futures::stream::Stream;
use hyper::{body, client::ResponseFuture, Body, StatusCode};

#[cfg(feature = "lz4")]
use crate::compression::lz4::Lz4Decoder;
use crate::{
    compression::Compression,
    error::{Error, Result},
};

pub enum Response {
    Waiting(ResponseFuture, Compression),
    Loading(Chunks),
}

impl Response {
    pub fn new(future: ResponseFuture, compression: Compression) -> Self {
        Self::Waiting(future, compression)
    }

    pub async fn resolve(&mut self) -> Result<&mut Chunks> {
        if let Self::Waiting(response, compression) = self {
            let response = response.await?;

            if response.status() != StatusCode::OK {
                let bytes = body::to_bytes(response.into_body()).await?;
                let reason = String::from_utf8_lossy(&bytes).trim().into();

                return Err(Error::BadResponse(reason));
            }

            let body = response.into_body();
            let chunks = match compression {
                Compression::None => Inner::Plain(body),
                #[cfg(feature = "lz4")]
                Compression::Lz4 => Inner::Lz4(Lz4Decoder::new(body)),
                #[cfg(feature = "gzip")]
                Compression::Gzip => Inner::Gzip(Box::new(GzipDecoder::new(BodyAdapter(body)))),
                #[cfg(feature = "zlib")]
                Compression::Zlib => Inner::Zlib(Box::new(ZlibDecoder::new(BodyAdapter(body)))),
                #[cfg(feature = "brotli")]
                Compression::Brotli => {
                    Inner::Brotli(Box::new(BrotliDecoder::new(BodyAdapter(body))))
                }
            };
            *self = Self::Loading(Chunks(chunks));
        }

        match self {
            Self::Waiting(..) => unreachable!(),
            Self::Loading(chunks) => Ok(chunks),
        }
    }
}

pub struct Chunks(Inner);

enum Inner {
    Plain(Body),
    #[cfg(feature = "lz4")]
    Lz4(Lz4Decoder<Body>),
    #[cfg(feature = "gzip")]
    Gzip(Box<GzipDecoder<BodyAdapter>>),
    #[cfg(feature = "zlib")]
    Zlib(Box<ZlibDecoder<BodyAdapter>>),
    #[cfg(feature = "brotli")]
    Brotli(Box<BrotliDecoder<BodyAdapter>>),
    Empty,
}

impl Stream for Chunks {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use Inner::*;
        let res = match self.0 {
            Plain(ref mut inner) => map_poll_err(Pin::new(inner).poll_next(cx), Into::into),
            #[cfg(feature = "lz4")]
            Lz4(ref mut inner) => Pin::new(inner).poll_next(cx),
            #[cfg(feature = "gzip")]
            Gzip(ref mut inner) => map_compression_poll(Pin::new(inner).poll_next(cx)),
            #[cfg(feature = "zlib")]
            Zlib(ref mut inner) => map_compression_poll(Pin::new(inner).poll_next(cx)),
            #[cfg(feature = "brotli")]
            Brotli(ref mut inner) => map_compression_poll(Pin::new(inner).poll_next(cx)),
            Empty => Poll::Ready(None),
        };

        if let Poll::Ready(None) = res {
            self.0 = Inner::Empty;
        }

        res
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        use Inner::*;
        match &self.0 {
            Plain(inner) => inner.size_hint(),
            #[cfg(feature = "lz4")]
            Lz4(inner) => inner.size_hint(),
            #[cfg(feature = "gzip")]
            Gzip(inner) => inner.size_hint(),
            #[cfg(feature = "zlib")]
            Zlib(inner) => inner.size_hint(),
            #[cfg(feature = "brotli")]
            Brotli(inner) => inner.size_hint(),
            Empty => (0, Some(0)),
        }
    }
}

#[cfg(any(feature = "gzip", feature = "zlib", feature = "brotli"))]
struct BodyAdapter(Body);

#[cfg(any(feature = "gzip", feature = "zlib", feature = "brotli"))]
impl Stream for BodyAdapter {
    type Item = std::io::Result<bytes_05::Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.0).poll_next(cx).map(|opt| {
            opt.map(|res| {
                res.map(to_bytes05)
                    .map_err(|err| Error::from(err).into_io())
            })
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

#[cfg(feature = "bytes-05")]
fn to_bytes05(bytes: Bytes) -> bytes_05::Bytes {
    bytes.to_vec().into()
}

#[cfg(feature = "bytes-05")]
fn from_bytes05(bytes: bytes_05::Bytes) -> Bytes {
    bytes.to_vec().into()
}

// XXX: https://github.com/rust-lang/rust/issues/63514
fn map_poll_err<T, E, E2>(
    poll: Poll<Option<Result<T, E>>>,
    f: impl FnOnce(E) -> E2,
) -> Poll<Option<Result<T, E2>>> {
    match poll {
        Poll::Ready(Some(Ok(val))) => Poll::Ready(Some(Ok(val))),
        Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(f(err)))),
        Poll::Ready(None) => Poll::Ready(None),
        Poll::Pending => Poll::Pending,
    }
}

#[cfg(any(feature = "gzip", feature = "zlib", feature = "brotli"))]
fn map_compression_poll(
    poll: Poll<Option<std::io::Result<bytes_05::Bytes>>>,
) -> Poll<Option<Result<Bytes>>> {
    match poll {
        Poll::Ready(Some(Ok(val))) => Poll::Ready(Some(Ok(from_bytes05(val)))),
        Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(Error::decode_io(err)))),
        Poll::Ready(None) => Poll::Ready(None),
        Poll::Pending => Poll::Pending,
    }
}
