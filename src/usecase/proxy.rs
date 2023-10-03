use crate::{
	config::{Configuration, Proxy},
	utils::{compression, control_headers, security_headers},
};

use hyper::{
	client::HttpConnector,
	http::{HeaderName, HeaderValue},
	Body, Client, HeaderMap, Method, Request, Response, StatusCode,
};
use hyper_rustls::HttpsConnector;

use std::{
	collections::HashSet,
	net::IpAddr,
	sync::{Arc, Mutex},
};

use super::static_file::compressed_static_files;

const IGNORE_CACHE: [&str; 3] = ["gzip", "deflate", "br"];

pub async fn mirror(
	req: Request<Body>,
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

	let default_ips = get_ips_from_string(&config.default_ip_whitelist);
	if default_ips.len() > 0 {
		whitelisted_ips.lock().unwrap().extend(default_ips);
	}

	if whitelisted_ips.lock().unwrap().len() > 0 {
		// If the IP is whitelisted, serve the request
		let forwarded_ips = get_ips_from_x_forwarded_for(&req);
		if !is_ip_in_whitelist(forwarded_ips, &whitelisted_ips.lock().unwrap()) {
			// return 403 shows what ip come from like "Forbidded from ip :123.23124.512"
			return Ok(Response::builder()
				.status(StatusCode::FORBIDDEN)
				.body("Forbidden".into())
				.unwrap());
		}
	}

	// Extract the path component from the incoming HTTP request's URI
	let path = req.uri().path();
	let method = req.method().clone();
	let headers = req.headers().clone();

	// Iterate over all HTTP servers defined in the configuration
	for server in &config.http.servers {
		// Iterate over all proxies defined for the current server
		for proxy in &server.proxies {
			// Check if the request URI's path starts with the current proxy's path
			if path.starts_with(&proxy.proxy_path) {
				return proxy_request(req, client, proxy, &headers, &method).await;
			}
		}

		let compressed = compressed_static_files(path, &server.root, &method, &headers).await;
		return compressed;
	}
	let result = handle(req).await;

	return result;
}

// Asynchronous function named 'handle'. It acts as a router for HTTP requests based on path
async fn proxy_request(
	req: Request<Body>,
	client: Client<HttpsConnector<HttpConnector>>,
	proxy: &Proxy,
	header: &HeaderMap<HeaderValue>,
	method: &Method,
) -> Result<Response<Body>, hyper::Error> {
	// Log the proxy
	let full_url = &proxy.proxy_pass.clone();
	let original_headers = req.headers().clone();
	let query_params = req.uri().query().unwrap_or("");

	// add uri with query params
	let path = req.uri().path();
	let sec_path = path.to_string();
	// add final path for replacement

	let final_path = path.replace(&proxy.proxy_path, "");
	let uri = match (proxy.retain_path, query_params.is_empty()) {
		(true, true) => format!("{}{}", full_url, final_path),
		(true, false) => format!("{}{}?{}", full_url, final_path, &query_params),
		(false, true) => format!("{}{}", full_url, &path),
		(false, false) => format!("{}{}?{}", full_url, &path, &query_params),
	};

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

	// Check if the proxy has custom request headers defined
	if let Some(request_headers) = &proxy.request_headers {
		// Iterate over each custom request header
		for request_header in request_headers {
			// Check if the header has key-value pairs defined
			if let Some(s_headers) = &request_header.headers {
				// Iterate over each key-value pair in the header
				for (key, value) in s_headers {
					// Convert the key to a HeaderName and the value to a HeaderValue
					let header_name = HeaderName::from_bytes(key.as_bytes()).unwrap();
					let header_value: HeaderValue = HeaderValue::from_str(value).unwrap();

					// Add the custom header to the request
					headers.insert(header_name, header_value);
				}
			}
		}
	}

	let res = client.request(request).await?;

	// check if response IGNORED_CACHED had in Content-Encoding
	// if yes, return res
	// if no, return compression::auto(&method, header, res)
	let encoding = res.headers().get("content-encoding");
	if let Some(encoding) = encoding {
		if let Ok(encoding) = encoding.to_str() {
			if IGNORE_CACHE.contains(&encoding) {
				return Ok(res);
			}
		}
	}

	// if let Some(encoding) = get_prefered_encoding(&res.headers()) {
	let mut res = compression::auto(&method, header, res).unwrap_or_else(|_| {
		Response::builder()
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.body("Internal Server Error".into())
			.unwrap()
	});

	// if cache-control is not set, set it
	if res.headers().get("cache-control").is_none() {
		control_headers::append_headers(&sec_path, &mut res);
	}
	security_headers::append_headers(&mut res);

	// cache control is no-cache, no-store, must-revalidate, max-age=0 do not append headers

	match res.headers().get("cache-control") {
		Some(cache_control) => {
			if let Ok(cache_control) = cache_control.to_str() {
				if cache_control.contains("no-cache")
					|| cache_control.contains("no-store")
					|| cache_control.contains("must-revalidate")
					|| cache_control.contains("max-age=0")
				{
					return Ok(res);
				}
			}
		}
		None => {
			control_headers::append_headers(&sec_path, &mut res);
		}
	}

	// if cache-control is set, append headers

	Ok(res)
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

fn get_ips_from_x_forwarded_for(req: &Request<hyper::Body>) -> Vec<IpAddr> {
	let mut ips = Vec::new();
	if let Some(header_value) = req.headers().get("X-Forwarded-For") {
		if let Ok(forwarded_for) = header_value.to_str() {
			for ip_str in forwarded_for.split(',').map(str::trim) {
				if let Ok(ip) = ip_str.parse::<IpAddr>() {
					ips.push(ip);
				}
			}
		}
	}
	ips
}

fn get_ips_from_string(ip_str: &str) -> Vec<IpAddr> {
	let mut ips = Vec::new();
	for ip_str in ip_str.split(',').map(str::trim) {
		if let Ok(ip) = ip_str.parse::<IpAddr>() {
			ips.push(ip);
		}
	}
	ips
}

fn is_ip_in_whitelist(ips: Vec<IpAddr>, whitelist: &HashSet<IpAddr>) -> bool {
	for ip in ips {
		if whitelist.contains(&ip) {
			return true;
		}
	}
	false
}
