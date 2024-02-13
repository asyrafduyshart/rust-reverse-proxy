use hyper::HeaderMap;

pub fn extract_specific_cookie_from_headermap(
	headers: &HeaderMap,
	cookie_name: &str,
) -> Option<String> {
	if let Some(cookie_header) = headers.get(hyper::header::COOKIE) {
		if let Ok(cookie_str) = cookie_header.to_str() {
			for cookie in cookie_str.split(';') {
				let mut parts = cookie.trim().splitn(2, '=');
				if let (Some(name), Some(value)) = (parts.next(), parts.next()) {
					if name == cookie_name {
						return Some(value.to_string());
					}
				}
			}
		}
	}
	None
}
