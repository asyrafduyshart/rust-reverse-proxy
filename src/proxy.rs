use hyper::{
	client::HttpConnector,
	http::{HeaderName, HeaderValue},
	Body, Client, HeaderMap, Request, Response, StatusCode,
};
use hyper_rustls::HttpsConnector;
use mime_guess::from_path;
use std::{
	collections::HashSet,
	net::IpAddr,
	sync::{Arc, Mutex},
};
use tokio::fs::read;

use crate::config::{Configuration, Proxy};

// Asynchronous function named 'proxy'. It acts as a router for HTTP requests based on path
pub async fn mirror(
	req: Request<Body>,
	socket: IpAddr,
	whitelisted_ips: Arc<Mutex<HashSet<IpAddr>>>,
	config: Arc<Configuration>,
) -> Result<Response<Body>, hyper::Error> {
	// Create a new HTTP client to send requests

	let https = hyper_rustls::HttpsConnectorBuilder::new()
		.with_native_roots()
		.https_or_http()
		.enable_http1()
		.build();

	let client = Client::builder().build::<_, hyper::Body>(https);

	if whitelisted_ips.lock().unwrap().len() > 0 {
		// If the IP is whitelisted, serve the request
		println!("request from ip: {}", socket);
		if !whitelisted_ips.lock().unwrap().contains(&socket) {
			return Ok(Response::builder()
				.status(StatusCode::FORBIDDEN)
				.body(Body::from("Forbidden"))
				.unwrap());
		}
	}

	// Extract the path component from the incoming HTTP request's URI
	let path = req.uri().path();

	// Iterate over all HTTP servers defined in the configuration
	for server in &config.http.servers {
		// Iterate over all proxies defined for the current server
		for proxy in &server.proxies {
			// Check if the request URI's path starts with the current proxy's path
			if path.starts_with(&proxy.proxy_path) {
				return proxy_request(req, client, proxy).await;
			}
		}
		// serve the static files
		return serve_static_files(path, &server.root).await;
	}
	handle(req).await
}

// Asynchronous function named 'handle'. It acts as a router for HTTP requests based on path
async fn proxy_request(
	req: Request<Body>,
	client: Client<HttpsConnector<HttpConnector>>,
	proxy: &Proxy,
) -> Result<Response<Body>, hyper::Error> {
	// Log the proxy
	let full_url = &proxy.proxy_pass.clone();
	let original_headers = req.headers().clone();
	let query_params = req.uri().query().unwrap_or("");

	// add uri with query params
	let path = req.uri().path();
	let uri = match (proxy.retain_path, query_params.is_empty()) {
		(true, true) => format!("{}", full_url),
		(true, false) => format!("{}?{}", full_url, &query_params),
		(false, true) => format!("{}{}", full_url, &path),
		(false, false) => format!("{}{}?{}", full_url, &path, &query_params),
	};

	println!("requesting to uri: {}", uri);

	let request_result = Request::builder()
		.method(req.method())
		.uri(&uri)
		.body(req.into_body());

	let mut request = match request_result {
		Ok(req) => req,
		Err(e) => {
			// handle the error here, perhaps logging it and returning a response indicating the error
			println!("Failed to construct the request: {}", e);
			return Ok(Response::builder()
				.status(StatusCode::INTERNAL_SERVER_ERROR)
				.body("Internal Server Error".into())
				.unwrap());
		}
	};

	// Get a mutable reference to the request's headers
	let headers: &mut HeaderMap<HeaderValue> = request.headers_mut();

	// Copy all the headers from the original request
	for (name, value) in original_headers {
		// Convert the key to a HeaderName and the value to a HeaderValue
		if let Some(header_name) = name {
			// Skip the host header
			if header_name == "host" {
				continue;
			}
			headers.insert(header_name, value);
		}
	}

	// print all proxy
	println!("proxy: {:?}", proxy);

	// Check if the proxy has custom request headers defined
	if let Some(request_headers) = &proxy.request_headers {
		println!("Adding custom request headers");
		println!("{:?}", request_headers);
		// Iterate over each custom request header
		for request_header in request_headers {
			// Check if the header has key-value pairs defined
			if let Some(s_headers) = &request_header.headers {
				// Iterate over each key-value pair in the header
				for (key, value) in s_headers {
					println!("{}: {}", key, value);
					// Convert the key to a HeaderName and the value to a HeaderValue
					let header_name = HeaderName::from_bytes(key.as_bytes()).unwrap();
					let header_value: HeaderValue = HeaderValue::from_str(value).unwrap();

					// Add the custom header to the request
					headers.insert(header_name, header_value);
				}
			}
		}
	}

	// Send the request using the client and return the response
	client.request(request).await
}

// List of web file extensions
const WEB_EXTENSIONS: [&str; 10] = [
	".html", ".js", ".css", ".png", ".jpg", ".jpeg", ".gif", ".svg", ".ico", ".json",
];

// Asynchronous function named 'serve_static_files'. It acts as a router for HTTP requests based on path
async fn serve_static_files(
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

// handler req that returns "not mapped" response
pub async fn handle(_req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
	let body = Body::from("not found");

	// Create a Response with a 400 Bad Request status code
	let response = Response::builder()
		.status(StatusCode::NOT_FOUND)
		.body(body)
		.unwrap();

	Ok(response)
}
