use http::header::{
	CONTENT_SECURITY_POLICY, STRICT_TRANSPORT_SECURITY, X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use hyper::{http, Body, Response};

/// It appends security headers like `Strict-Transport-Security: max-age=63072000; includeSubDomains; preload` (2 years max-age),
///`X-Frame-Options: DENY` and `Content-Security-Policy: frame-ancestors 'self'`.
pub fn append_headers(resp: &mut Response<Body>) {
	// Strict-Transport-Security (HSTS)
	resp.headers_mut().insert(
		STRICT_TRANSPORT_SECURITY,
		"max-age=63072000; includeSubDomains; preload"
			.parse()
			.unwrap(),
	);

	// X-Frame-Options
	resp.headers_mut()
		.insert(X_FRAME_OPTIONS, "DENY".parse().unwrap());

	// X-Content-Type-Options
	resp.headers_mut()
		.insert(X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());

	// Content Security Policy (CSP)
	resp.headers_mut().insert(
		CONTENT_SECURITY_POLICY,
		"frame-ancestors 'self'".parse().unwrap(),
	);
}
