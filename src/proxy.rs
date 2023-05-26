use std::{sync::Arc};
use hyper::{Body, Request, Response, Client, HeaderMap, http::{HeaderValue, HeaderName}, StatusCode, client::HttpConnector};
use hyper_tls::HttpsConnector;
use tokio::fs::read;

use crate::config::{Configuration, Proxy};

// Asynchronous function named 'proxy'. It acts as a router for HTTP requests based on path
pub async fn mirror(req: Request<Body>, config: Arc<Configuration>) -> Result<Response<Body>, hyper::Error> {
    
    // Create a new HTTP client to send requests
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);

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
async fn proxy_request(req: Request<Body>, client: Client<HttpsConnector<HttpConnector>>, proxy: &Proxy) -> Result<Response<Body>, hyper::Error> {
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
        },
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

    // Send the request using the client and return the response
    client.request(request).await
}


// List of web file extensions
const WEB_EXTENSIONS: [&str; 9] = [
    ".html", 
    ".js", 
    ".css", 
    ".png", 
    ".jpg", 
    ".jpeg", 
    ".gif", 
    ".svg", 
    ".ico"
];

// Asynchronous function named 'serve_static_files'. It acts as a router for HTTP requests based on path
async fn serve_static_files(path: &str, folder_path: &String) -> Result<Response<Body>, hyper::Error> {
    // Check if the path has a web file extension
    let has_web_extension = WEB_EXTENSIONS.iter().any(|ext| path.ends_with(ext));
    let file_path = if has_web_extension {
        format!("{}{}", folder_path, path)
    } else {
        //format with added folder path
        format!("{}/index.html", folder_path)
        // "static/index.html".to_string()
    };

    match read(&file_path).await {
        Ok(bytes) => {
            let body = Body::from(bytes);
            Ok(Response::new(body))
        },
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
            let bytes = read(back_path).await.unwrap_or_else(|e| {
                eprintln!("Failed to read fallback file {}/index.html: {}", folder_path, e);
                Vec::new()  // Empty Vec
            });
            let body = Body::from(bytes);
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(body)
                .unwrap())
        }
    }
}


// handler req that returns "not mapped" response
pub async fn handle(_req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let body = Body::from("not found");

    // Create a Response with a 400 Bad Request status code
    let response = Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(body).unwrap();

    Ok(response)
}