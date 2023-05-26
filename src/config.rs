use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequestHeader {
    #[serde(flatten)]
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Proxy {
    pub proxy_pass: String,
    pub proxy_path: String,
    pub retain_path: bool,
    #[serde(default)]
    pub request_headers: Option<Vec<RequestHeader>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    pub root: String,
    pub name: String,
    pub proxies: Vec<Proxy>,
    pub listen: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Http {
    pub servers: Vec<Server>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Configuration {
    // optional fields
    #[serde(default)]
    pub log_level: String,
    #[serde(default)]
    pub ip_check_interval: String,
    #[serde(default)]
    pub ip_whitelist_url: String,
    #[serde(default)]
    pub default_ip_whitelist: String,
    pub http: Http,
}
