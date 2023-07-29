use hyper::{http::HeaderValue, Body, Response, StatusCode};

use mime_guess::from_path;
use tokio::fs::read;

// List of web file extensions
const WEB_EXTENSIONS: [&str; 11] = [
	".html", ".js", ".css", ".png", ".jpg", ".jpeg", ".gif", ".svg", ".ico", ".json", ".mp3",
];

// Asynchronous function named 'serve_static_files'. It acts as a router for HTTP requests based on path
pub async fn serve_static_files(
	path: &str,
	folder_path: &String,
) -> Result<Response<Body>, hyper::Error> {
	// Check if the path has a web file extension
	let has_web_extension = WEB_EXTENSIONS.iter().any(|ext| path.ends_with(ext));
	let file_path = if has_web_extension {
		format!("{}{}", folder_path, path)
	} else {
		// format with added folder path
		format!("{}/index.html", folder_path)
	};

	match read(&file_path).await {
		Ok(bytes) => {
			let mime_type = from_path(&file_path).first_or_octet_stream();
			let mut response = Response::new(Body::from(bytes));
			response.headers_mut().insert(
				hyper::header::CONTENT_TYPE,
				HeaderValue::from_str(mime_type.as_ref()).unwrap(),
			);
			Ok(response)
		}
		Err(e) => {
			if has_web_extension {
				// If it was a specific file and it failed to read, log the error
				eprintln!("Failed to read file {}: {}", file_path, e);
				// return response not found
				return Ok(Response::builder()
					.status(StatusCode::NOT_FOUND)
					.body(Body::from("Not Found"))
					.unwrap());
			}

			// In case of an error or non-web extension path, fallback to index.html
			// format the path with folder path
			let back_path = format!("{}/index.html", folder_path);
			let bytes = read(&back_path).await.unwrap_or_else(|e| {
				eprintln!(
					"Failed to read fallback file {}/index.html: {}",
					folder_path, e
				);
				// return text no file found in string vec
				b"File not found".to_vec()
			});
			let mime_type = from_path(&back_path).first_or_octet_stream();
			let mut response = Response::new(Body::from(bytes));
			response.headers_mut().insert(
				hyper::header::CONTENT_TYPE,
				HeaderValue::from_str(mime_type.as_ref()).unwrap(),
			);
			Ok(response)
		}
	}
}
