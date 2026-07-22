/// Safari web extension / deep-link bridge for iOS.
///
/// On iOS, Safari web extensions can communicate with their native
/// containing app via:
/// 1. Native messaging (Safari app extension → containing app)
/// 2. URL scheme deep links (nostrsigner://...)
///
/// This module handles the URL scheme-based deep link protocol.

use serde::{Deserialize, Serialize};

/// A deep link request received via the `nostrsigner://` URL scheme.
///
/// Format: nostrsigner://[method]?param1=value1&param2=value2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepLinkRequest {
    /// The method (path component of the URL)
    pub method: String,
    /// Query parameters
    pub params: std::collections::HashMap<String, String>,
}

/// A response to a deep link request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepLinkResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl DeepLinkRequest {
    /// Parse a deep link URL into a request.
    ///
    /// Expected format: nostrsigner://sign_event?event=...&pubkey=...
    pub fn from_url(url: &str) -> Result<Self, String> {
        let url = url.strip_prefix("nostrsigner://").ok_or_else(|| {
            format!("invalid scheme: expected nostrsigner://, got {}", url)
        })?;

        let mut parts = url.splitn(2, '?');
        let method = parts.next().unwrap_or("").to_string();
        let query = parts.next().unwrap_or("");

        let mut params = std::collections::HashMap::new();
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                params.insert(
                    urlencoding_decode(key),
                    urlencoding_decode(value),
                );
            }
        }

        Ok(Self { method, params })
    }
}

fn urlencoding_decode(s: &str) -> String {
    s.replace('+', " ")
        .replace("%20", " ")
        .replace("%3A", ":")
        .replace("%2F", "/")
        .replace("%3F", "?")
        .replace("%3D", "=")
        .replace("%26", "&")
}

/// Supported deep link methods
pub const DL_METHOD_SIGN_EVENT: &str = "sign_event";
pub const DL_METHOD_GET_PUBLIC_KEY: &str = "get_public_key";
pub const DL_METHOD_NIP44_ENCRYPT: &str = "nip44_encrypt";
pub const DL_METHOD_NIP44_DECRYPT: &str = "nip44_decrypt";

/// The URL scheme registered for this app
pub const DEEP_LINK_SCHEME: &str = "nostrsigner";

/// Construct a deep link URL for returning the response to the caller
pub fn build_response_url(original_url: &str, response: &DeepLinkResponse) -> String {
    let response_json = serde_json::to_string(response).unwrap_or_default();
    let encoded = urlencoding_encode(&response_json);
    format!("{}://callback?response={}", original_url, encoded)
}

fn urlencoding_encode(s: &str) -> String {
    s.replace(' ', "%20")
        .replace(':', "%3A")
        .replace('/', "%2F")
        .replace('?', "%3F")
        .replace('=', "%3D")
        .replace('&', "%26")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sign_event_deep_link() {
        let url = "nostrsigner://sign_event?event=%7B%22kind%22%3A1%7D&pubkey=abc123";
        let req = DeepLinkRequest::from_url(url).unwrap();
        assert_eq!(req.method, "sign_event");
        assert_eq!(req.params.get("pubkey").unwrap(), "abc123");
    }

    #[test]
    fn test_parse_get_public_key() {
        let url = "nostrsigner://get_public_key";
        let req = DeepLinkRequest::from_url(url).unwrap();
        assert_eq!(req.method, "get_public_key");
        assert!(req.params.is_empty());
    }

    #[test]
    fn test_build_response_url() {
        let resp = DeepLinkResponse {
            result: Some("pubkey-hex".into()),
            error: None,
        };
        let url = build_response_url("nostrsigner://callback", &resp);
        assert!(url.contains("response"));
        assert!(url.contains("pubkey-hex"));
    }

    #[test]
    fn test_invalid_scheme() {
        let result = DeepLinkRequest::from_url("https://example.com");
        assert!(result.is_err());
    }
}
