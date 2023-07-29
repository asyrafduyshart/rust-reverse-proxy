use std::{io::Cursor, pin::Pin, task::Poll};

use futures::Stream;
use hyper::body::Bytes;
use serde_json::{StreamDeserializer, Value};

pub struct PrintingStream<S: Stream> {
	pub inner: S,
	pub buffer: Vec<u8>,
}

impl<S: Stream<Item = Result<Bytes, hyper::Error>> + Unpin> Stream for PrintingStream<S> {
	type Item = Result<Bytes, hyper::Error>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> Poll<Option<Self::Item>> {
		let next = Pin::new(&mut self.inner).poll_next(cx);
		dbg!(&next); // Add this line
		match futures::ready!(next) {
			Some(Ok(chunk)) => {
				self.buffer.extend_from_slice(&chunk);
				Poll::Ready(Some(Ok(chunk)))
			}
			Some(Err(e)) => {
				println!("error: {:?}", e);
				Poll::Ready(Some(Err(e)))
			}
			None => {
				println!("stream is done");
				let s = String::from_utf8_lossy(&self.buffer);
				println!("buffer: {}", s);
				Poll::Ready(None)
			}
		}
	}
}

pub struct JsonPrintingStream<S: Stream> {
	pub inner: S,
	pub buffer: Vec<u8>,
}

impl<S: Stream<Item = Result<Bytes, hyper::Error>> + Unpin> Stream for JsonPrintingStream<S> {
	type Item = Result<Bytes, hyper::Error>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> Poll<Option<Self::Item>> {
		let next = Pin::new(&mut self.inner).poll_next(cx);
		match futures::ready!(next) {
			Some(Ok(chunk)) => {
				self.buffer.extend_from_slice(&chunk);
				let mut de =
					StreamDeserializer::<serde_json::de::IoRead<Cursor<Vec<u8>>>, Value>::new(
						serde_json::de::IoRead::new(Cursor::new(self.buffer.clone())),
					);
				while let Some(Ok(json)) = de.next() {
					println!("Received JSON: {}\n", json);
				}
				Poll::Ready(Some(Ok(chunk)))
			}
			Some(Err(e)) => {
				println!("error: {:?}", e);
				Poll::Ready(Some(Err(e)))
			}
			None => {
				println!("stream is done");
				let s = String::from_utf8_lossy(&self.buffer);
				println!("buffer: {}", s);
				Poll::Ready(None)
			}
		}
	}
}
