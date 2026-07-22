use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip46Request {
    pub id: String,
    pub method: String,
    pub params: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip46Response {
    pub id: String,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip46ConnectRequest {
    pub relay: String,
    pub client_name: Option<String>,
    pub client_url: Option<String>,
    pub client_icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip46ConnectResponse {
    pub bunker_pubkey: String,
    pub relay: String,
}
