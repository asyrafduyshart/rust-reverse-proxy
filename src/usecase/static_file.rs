use std::fs;

use hyper::{header, http::HeaderValue, Body, HeaderMap, Method, Response, StatusCode};
use lol_html::{html_content::ContentType, HtmlRewriter, Settings};
use mime_guess::from_path;
use tokio::fs::read;

use crate::utils::{compression, control_headers, security_headers};

// List of web file extensions
const WEB_EXTENSIONS: [&str; 11] = [
	".html", ".js", ".css", ".png", ".jpg", ".jpeg", ".gif", ".svg", ".ico", ".json", ".mp3",
];

// Asynchronous function named 'serve_static_files'. It acts as a router for HTTP requests based on path
pub async fn compressed_static_files(
	path: &str,
	folder_path: &String,
	method: &Method,
	headers: &HeaderMap<HeaderValue>,
	scripts: Vec<&str>,
	onloadfunction: Option<&String>,
) -> Result<Response<Body>, hyper::Error> {
	let has_web_extension = WEB_EXTENSIONS.iter().any(|ext| path.ends_with(ext));
	let file_path = if has_web_extension {
		format!("{}{}", folder_path, path)
	} else {
		format!("{}/index.html", folder_path)
	};
	let file_check = &file_path.clone();

	if file_check.ends_with(".html") {
		// if the file path is html, serve the html with the script
		Ok(serve_html_with_scripts(&file_path, scripts, onloadfunction))
	} else {
		// if the file path is not html, just return the file
		Ok(serve_default_static(path, file_check, method, headers).await)
	}
	// if the file path is not html, just return the file
}

// Asynchronous function named 'serve_static_files'. It acts as a router for HTTP requests based on path
#[allow(dead_code)]
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

fn serve_html_with_scripts(
	file_path: &String,
	scripts: Vec<&str>,
	onloadfunction: Option<&String>,
) -> Response<Body> {
	let html = match fs::read_to_string(file_path) {
		Ok(html) => html,
		Err(e) => {
			println!("Failed to read file: {}", e);
			return Response::builder()
				.status(StatusCode::NOT_FOUND)
				.body("Not Found".into())
				.unwrap();
		}
	};

	let mut output_buffer: Vec<u8> = Vec::new();
	let mut rewriter = HtmlRewriter::new(
		Settings {
			element_content_handlers: vec![lol_html::element!("body", |el| {
				for script in &scripts {
					el.before(script, ContentType::Html);
				}

				// Add onload function to the body tag if any
				if let Some(onload) = onloadfunction {
					if el.get_attribute("onload").is_some() {
						// If the attribute exists, you might want to append your function to it or replace it
						// Here's how to append
						let existing_onload = el.get_attribute("onload").unwrap_or_default();
						let new_onload = format!("{} {}", existing_onload, onload);
						el.set_attribute("onload", &new_onload).unwrap();
					} else {
						// If the "onload" attribute doesn't exist, set it
						el.set_attribute("onload", onload).unwrap();
					}
				}
				Ok(())
			})],
			..Settings::default()
		},
		|c: &[u8]| {
			output_buffer.extend_from_slice(c);
		},
	);
	rewriter.write(html.as_bytes()).unwrap();
	rewriter.end().unwrap();

	let mime_type = from_path(&file_path).first_or_octet_stream();
	let mime_str = mime_type.as_ref();
	let mut response = Response::new(Body::from(output_buffer));
	response.headers_mut().insert(
		header::CONTENT_TYPE,
		header::HeaderValue::from_str(mime_str).unwrap(),
	);
	response
}

async fn serve_default_static(
	path: &str,
	file_check: &String,
	method: &Method,
	headers: &HeaderMap<HeaderValue>,
) -> Response<Body> {
	match read(file_check).await {
		Ok(bytes) => {
			let mime_type = from_path(&file_check).first_or_octet_stream();
			let mime_str = mime_type.as_ref();

			// check if the MIME type is compressible
			// if no supported compression type found or the MIME type is not compressible, just return the file
			let mut response = compression::auto(method, headers, Response::new(Body::from(bytes)))
				.unwrap_or_else(|_| {
					Response::builder()
						.status(StatusCode::INTERNAL_SERVER_ERROR)
						.body("Internal Server Error".into())
						.unwrap()
				});

			response.headers_mut().insert(
				header::CONTENT_TYPE,
				header::HeaderValue::from_str(mime_str).unwrap(),
			);

			control_headers::append_headers(path, &mut response);
			security_headers::append_headers(&mut response);

			response
		}
		Err(e) => {
			println!("Failed to read file: {}", e);
			Response::builder()
				.status(StatusCode::NOT_FOUND)
				.body("Not Found".into())
				.unwrap()
		}
	}
}
