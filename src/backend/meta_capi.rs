//! Meta Conversions API (CAPI) client for Business Messaging.
//!
//! Submits conversion events (e.g. Purchase, Lead) directly to the Meta Conversions API
//! using the Click-to-WhatsApp click identifier (`ctwa_clid`) and WhatsApp Business Account ID.
//!
//! Conforming to official Meta Business Messaging CAPI documentation:
//! - Conversions require `ctwa_clid` to attribute to Click-to-WhatsApp ads.
//! - Access token is transmitted securely via `Authorization: Bearer` header, never in URL parameters.
//! - Tokens are strictly protected and never written to logs.
//! - Events include `event_id` to ensure server-side deduplication.

use sha2::{Digest, Sha256};

/// Normalizes a customer phone number into international digits and computes its SHA-256 hash.
pub fn hash_phone(phone: &str) -> String {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    let normalized = digits.trim_start_matches('+').to_lowercase();
    let digest = Sha256::digest(normalized.as_bytes());
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

/// Result returned after submitting a conversion event to Meta Graph API.
#[derive(Clone, Debug)]
pub struct MetaConversionOutcome {
    pub success: bool,
    pub event_name: String,
    pub events_received: usize,
    pub fbtrace_id: Option<String>,
    pub message: String,
    pub raw_response: String,
}

/// Dispatches a conversion event to Meta Conversions API via Graph API v21.0.
///
/// Follows Meta Business Messaging CAPI specifications:
/// - Rejects dispatch without `ctwa_clid`.
/// - Never includes the access token in URL query parameters.
/// - Never logs sensitive credentials.
pub fn send_conversion(
    token: &str,
    dataset_id: &str,
    waba_id: Option<&str>,
    test_event_code: Option<&str>,
    _chat_id: &str,
    phone: &str,
    value_cents: i64,
    ctwa_clid: Option<&str>,
    custom_event_name: Option<&str>,
    event_id: Option<&str>,
) -> Result<MetaConversionOutcome, String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("Token de Acesso do Meta não configurado".to_string());
    }

    let dataset_id = dataset_id.trim();
    if dataset_id.is_empty() {
        return Err(
            "Identificador do Pixel / Conjunto de Dados (Dataset ID) não configurado".to_string(),
        );
    }

    // Official Meta requirement: Conversions for Business Messaging require ctwa_clid
    let clid = match ctwa_clid {
        Some(c) if !c.trim().is_empty() => c.trim(),
        _ => {
            return Err("Envio recusado: Meta Conversions API para Business Messaging exige ctwa_clid (clique rastreado).".to_string());
        }
    };

    let event_name = custom_event_name
        .filter(|s| !s.is_empty())
        .unwrap_or("Purchase");

    let now_unix = crate::util::now();
    let hashed_ph = hash_phone(phone);
    let value_reais = value_cents as f64 / 100.0;

    // Build user_data
    let mut user_data = serde_json::json!({
        "ph": [hashed_ph],
        "ctwa_clid": clid,
    });
    if let Some(waba) = waba_id.filter(|w| !w.trim().is_empty()) {
        user_data["whatsapp_business_account_id"] = serde_json::Value::String(waba.trim().to_string());
    }

    // Build custom_data
    let custom_data = serde_json::json!({
        "currency": "BRL",
        "value": value_reais,
        "attribution_type": "direct_ctwa",
    });

    // Build event object with deduplication event_id
    let mut event_obj = serde_json::json!({
        "event_name": event_name,
        "event_time": now_unix,
        "action_source": "business_messaging",
        "messaging_channel": "whatsapp",
        "user_data": user_data,
        "custom_data": custom_data,
    });

    if let Some(eid) = event_id.filter(|e| !e.trim().is_empty()) {
        event_obj["event_id"] = serde_json::Value::String(eid.trim().to_string());
    }

    let mut payload = serde_json::json!({
        "data": [event_obj],
    });

    // Optional test_event_code for Meta Events Manager testing
    if let Some(code) = test_event_code.filter(|c| !c.trim().is_empty()) {
        payload["test_event_code"] = serde_json::Value::String(code.trim().to_string());
    }

    // Token is securely sent in the Authorization header, NEVER in the URL
    let url = format!("https://graph.facebook.com/v21.0/{}/events", dataset_id);
    let payload_str = payload.to_string();

    let agent = crate::proxy::agent();
    let response = agent
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .send(payload_str.as_bytes());

    match response {
        Ok(mut resp) => {
            let status = resp.status();
            let body_str = resp.body_mut().read_to_string().unwrap_or_default();
            let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap_or_default();

            if status.as_u16() >= 200 && status.as_u16() < 300 {
                let events_received = parsed["events_received"].as_u64().unwrap_or(1) as usize;
                let fbtrace_id = parsed["fbtrace_id"].as_str().map(ToString::to_string);
                let message = format!(
                    "Conversão '{}' (R$ {:.2}) enviada ao Meta com sucesso",
                    event_name, value_reais
                );
                Ok(MetaConversionOutcome {
                    success: true,
                    event_name: event_name.to_string(),
                    events_received,
                    fbtrace_id,
                    message,
                    raw_response: body_str,
                })
            } else {
                let err_msg = parsed["error"]["message"]
                    .as_str()
                    .unwrap_or("Erro retornado pelo Meta");
                let err_type = parsed["error"]["type"].as_str().unwrap_or("");
                let err_code = parsed["error"]["code"].as_i64().unwrap_or(0);
                Err(format!(
                    "Meta Graph API (HTTP {} / code {}): {} ({})",
                    status, err_code, err_msg, err_type
                ))
            }
        }
        Err(err) => Err(format!("Falha na conexão com a API do Meta: {err}")),
    }
}

/// Sends a test event with the test_event_code to validate connectivity and display exact response.
pub fn test_connection(
    token: &str,
    dataset_id: &str,
    waba_id: Option<&str>,
    test_event_code: Option<&str>,
) -> Result<MetaConversionOutcome, String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("Token de Acesso do Meta não informado".to_string());
    }
    let dataset_id = dataset_id.trim();
    if dataset_id.is_empty() {
        return Err("Dataset / Pixel ID não informado".to_string());
    }

    let now_unix = crate::util::now();
    let dummy_hash = hash_phone("5511999999999");
    let mut user_data = serde_json::json!({
        "ph": [dummy_hash],
        "ctwa_clid": "TEST_CTWA_CLID_VALIDATION",
    });
    if let Some(waba) = waba_id.filter(|w| !w.trim().is_empty()) {
        user_data["whatsapp_business_account_id"] = serde_json::Value::String(waba.trim().to_string());
    }

    let event_obj = serde_json::json!({
        "event_name": "TestEvent",
        "event_time": now_unix,
        "action_source": "business_messaging",
        "messaging_channel": "whatsapp",
        "user_data": user_data,
        "custom_data": {
            "currency": "BRL",
            "value": 1.00,
            "test_mode": true
        },
        "event_id": format!("test_{}", now_unix),
    });

    let mut payload = serde_json::json!({
        "data": [event_obj],
    });

    if let Some(code) = test_event_code.filter(|c| !c.trim().is_empty()) {
        payload["test_event_code"] = serde_json::Value::String(code.trim().to_string());
    }

    let url = format!("https://graph.facebook.com/v21.0/{}/events", dataset_id);
    let payload_str = payload.to_string();

    let agent = crate::proxy::agent();
    let response = agent
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .send(payload_str.as_bytes());

    match response {
        Ok(mut resp) => {
            let status = resp.status();
            let body_str = resp.body_mut().read_to_string().unwrap_or_default();
            let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap_or_default();

            if status.as_u16() >= 200 && status.as_u16() < 300 {
                let events_received = parsed["events_received"].as_u64().unwrap_or(1) as usize;
                let fbtrace_id = parsed["fbtrace_id"].as_str().map(ToString::to_string);
                Ok(MetaConversionOutcome {
                    success: true,
                    event_name: "TestEvent".to_string(),
                    events_received,
                    fbtrace_id: fbtrace_id.clone(),
                    message: format!(
                        "Conexão com Meta validada! HTTP {} ({} evento recebido, trace: {})",
                        status, events_received, fbtrace_id.as_deref().unwrap_or("n/a")
                    ),
                    raw_response: body_str,
                })
            } else {
                let err_msg = parsed["error"]["message"]
                    .as_str()
                    .unwrap_or("Erro retornado pelo Meta");
                let err_type = parsed["error"]["type"].as_str().unwrap_or("");
                let err_code = parsed["error"]["code"].as_i64().unwrap_or(0);
                Err(format!(
                    "HTTP {} (Code {} / {}): {}\nResposta: {}",
                    status, err_code, err_type, err_msg, body_str
                ))
            }
        }
        Err(err) => Err(format!("Falha na conexão com os servidores da Meta: {err}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phone_hashing() {
        let phone1 = "+55 (11) 99999-8888";
        let phone2 = "5511999998888";
        assert_eq!(hash_phone(phone1), hash_phone(phone2));
        assert_eq!(hash_phone(phone1).len(), 64);
    }
}
