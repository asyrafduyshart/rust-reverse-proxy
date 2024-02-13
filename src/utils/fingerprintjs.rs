use base64::{engine::general_purpose, Engine as _};

use serde::{Deserialize, Serialize};
use serde_json::from_slice;

#[derive(Serialize, Deserialize, Debug)]
pub struct CookieData {
	#[serde(rename = "visitorId")]
	pub visitor_id: String,
	#[serde(rename = "requestId")]
	pub request_id: String,
}

pub static FP_SCRIPT: &str = r#"
<script>
    // Initialize the agent at application startup.
    // Some ad blockers or browsers will block Fingerprint CDN URL.
    // To fix this, please use the NPM package instead.
    
    const fpPromise = import('/js/fp.js')
        .then(FingerprintJS => FingerprintJS.load())
        .catch(e => {
            console.error("ERROR",e);
        
        });

    // Get the visitor identifier when you need it.

    let fingerprint = {};

    fpPromise
        .then(fp => fp.get())
        .then(result => {
            // This is the visitor identifier:
            fingerprint = result;
        })
        .catch(e => {
            if (e.message === "Network connection error") {
                function createPersistentModal() {
                    // Create the overlay div
                    var overlay = document.createElement('div');
                    overlay.style.position = 'fixed';
                    overlay.style.top = '0';
                    overlay.style.left = '0';
                    overlay.style.width = '100%';
                    overlay.style.height = '100%';
                    overlay.style.backgroundColor = 'rgba(0,0,0,0.5)';
                    overlay.style.zIndex = '1000';
                    overlay.style.display = 'flex';
                    overlay.style.justifyContent = 'center';
                    overlay.style.alignItems = 'center';
        
                    // Create the modal content div
                    var modalContent = document.createElement('div');
                    modalContent.style.backgroundColor = '#fff';
                    modalContent.style.padding = '20px';
                    modalContent.style.borderRadius = '5px';
                    modalContent.style.boxShadow = '0 4px 8px rgba(0,0,0,0.1)';
                    modalContent.innerText = "Please Disable Ad Blocker To Continue";
        
                    // Append the modal content to the overlay
                    overlay.appendChild(modalContent);
        
                    // Append the overlay to the body
                    document.body.appendChild(overlay);
                }
        
                // Call the function to create and show the modal
                createPersistentModal();
            }
        });

        function setCookie(name, value, daysToLive) {
            // Encode value in order to escape semicolons, commas, and whitespace
            // convert fingerprint to base64

            const { visitorId, requestId } = value;
            if (!visitorId || !requestId) return;

            let fp = btoa(JSON.stringify({ visitorId, requestId }));

            let cookie = name + "=" + encodeURIComponent(fp);

            if (typeof daysToLive === "number") {
                /* Sets the max-age attribute so the cookie expires
                   after the specified number of days */
                cookie += "; max-age=" + (daysToLive*24*60*60);
                
                // Secure attribute for HTTPS only
                cookie += "; secure";

                // SameSite attribute for CSRF protection
                cookie += "; samesite=strict";
            }
            cookie += "; path=/";

            document.cookie = cookie;
        }

        // Example: Set a cookie that expires in 7 days
        function initFpCookie() {
            setCookie("_fp_id", fingerprint , 7);
        }
    </script>
"#;

pub fn parse_cookie(cookie_value: &str) -> Result<CookieData, Box<dyn std::error::Error>> {
	// Replace URL encoded base64 padding if present and decode
	// let decoded_bytes = decode_engine(cookie_value.replace("%3D", "=").as_bytes(), &URL_SAFE)?;
	let decoded_bytes = general_purpose::URL_SAFE.decode(cookie_value.as_bytes())?;
	// Deserialize the JSON into the Rust struct
	let data: CookieData = from_slice(&decoded_bytes)?;
	Ok(data)
}
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_cookie() {
		// Test case 1: Valid cookie value
		let cookie_value =
			"eyJ2aXNpdG9ySWQiOiAiMTIzNDU2Nzg5MCIsICJyZXF1ZXN0SWQiOiAiMTIzNDU2Nzg5MCJ9";
		let result = parse_cookie(cookie_value);
		assert!(result.is_ok());
		let data = result.unwrap();
		assert_eq!(data.visitor_id, "1234567890");
		assert_eq!(data.request_id, "1234567890");

		// Test case 2: Invalid cookie value
		let cookie_value = "invalid_cookie_value";
		let result = parse_cookie(cookie_value);
		assert!(result.is_err());
	}
}
