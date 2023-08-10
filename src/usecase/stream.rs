use futures::Stream;
use hyper::body::Bytes;

use serde_json::{StreamDeserializer, Value};

use std::io::Cursor;
use std::{pin::Pin, task::Poll};

pub struct JsonPrintingStream<S: Stream> {
	pub inner: S,
	pub buffer: Vec<u8>,
}

// Stream JsonPrintingStream to take data asyncronously as it received and send to client
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

				while let Some(Ok(_json)) = de.next() {
					// println!("Received JSON: {}", json);
					// stop after first json
				}
				return Poll::Ready(Some(Ok(chunk)));
			}
			Some(Err(e)) => {
				log::error!("Error while reading stream: {}", e);
				Poll::Ready(Some(Err(e)))
			}
			None => Poll::Ready(None),
		}
	}
}
