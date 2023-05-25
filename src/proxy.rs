use std::sync::Arc;

use hyper::{Body, Request, Response, Client, HeaderMap, http::{HeaderValue, HeaderName}, StatusCode};
use hyper_tls::HttpsConnector;

use crate::config::Configuration;

// Asynchronous function named 'proxy'. It acts as a router for HTTP requests based on path
pub async fn mirror(req: Request<Body>, config: Arc<Configuration>) -> Result<Response<Body>, hyper::Error> {
    
    // Create a new HTTP client to send requests
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);
    
    // Extract the path component from the incoming HTTP request's URI
    let path = req.uri().path();
    let query_params = req.uri().query().unwrap_or("");
    // Iterate over all HTTP servers defined in the configuration
    for server in &config.http.servers {

        // Iterate over all proxies defined for the current server
        for proxy in &server.proxies {

            // Check if the request URI's path starts with the current proxy's path
            if path.starts_with(&proxy.proxy_path) {

                // Log the matched path
                println!("Proxy Path: {}", path);

                // add uri with query params
                let uri = format!("{}{}?{}", &proxy.proxy_pass, &path, &query_params);
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

                // Check if the proxy has custom request headers defined
                if let Some(request_headers) = &proxy.request_headers {

                    // Iterate over each custom request header
                    for request_header in request_headers{

                        // Check if the header has key-value pairs defined
                        if let Some(s_headers) = &request_header.headers {

                            // Iterate over each key-value pair in the header
                            for (key, value) in s_headers {

                                // Convert the key to a HeaderName and the value to a HeaderValue
                                let header_name = HeaderName::from_bytes(key.as_bytes()).unwrap();
                                let header_value = HeaderValue::from_str(value).unwrap();

                                // Add the custom header to the request
                                headers.insert(header_name, header_value);
                            }
                        }
                    }
                }

                // Send the request using the client and return the response
                return client.request(request).await;
            }
        }
    }

    // If none of the proxies in the configuration matches the path of the incoming request,
    // handle the request using the default handler
    handle(req).await
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