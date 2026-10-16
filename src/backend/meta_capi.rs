//! Meta Conversions API (CAPI) client for Business Messaging.
//!
//! Submits conversion events (e.g. Purchase, Lead) directly to the Meta Conversions API
//! using the Click-to-WhatsApp click identifier (`ctwa_clid`) or privacy-safe hashed phone numbers.
//!
//! For leads that arrived via Plan B (text rules without direct `ctwa_clid`), events are
//! explicitly flagged as `plan_b_experimental` attribution, as required by Meta specifications.

use sha2::{Digest, Sha256};

/// Normalizes a customer phone number into international E.164 digits and computes its SHA-256 hash.
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
/// Runs synchronously within the caller's thread; callers on the UI thread should spawn
/// a thread or tokio task to prevent blocking the frame loop.
pub fn send_conversion(
    token: &str,
    dataset_id: &str,
    test_event_code: Option<&str>,
    _chat_id: &str,
    phone: &str,
    value_cents: i64,
    ctwa_clid: Option<&str>,
    custom_event_name: Option<&str>,
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

    let event_name = custom_event_name
        .filter(|s| !s.is_empty())
        .unwrap_or("Purchase");

    let now_unix = crate::util::now();
    let hashed_ph = hash_phone(phone);
    let value_reais = value_cents as f64 / 100.0;

    let has_clid = ctwa_clid.is_some_and(|c| !c.trim().is_empty());

    // Build user_data
    let mut user_data = serde_json::json!({
        "ph": [hashed_ph]
    });
    if let Some(clid) = ctwa_clid.filter(|c| !c.trim().is_empty()) {
        user_data["ctwa_clid"] = serde_json::Value::String(clid.to_string());
    }

    // Build custom_data
    let mut custom_data = serde_json::json!({
        "currency": "BRL",
        "value": value_reais,
    });
    if has_clid {
        custom_data["attribution_type"] = serde_json::Value::String("direct_ctwa".to_string());
    } else {
        custom_data["attribution_type"] =
            serde_json::Value::String("plan_b_experimental".to_string());
        custom_data["note"] = serde_json::Value::String(
            "Atribuição Plano B por regras de texto sem ctwa_clid".to_string(),
        );
    }

    // Build event object
    let event_obj = serde_json::json!({
        "event_name": event_name,
        "event_time": now_unix,
        "action_source": "business_messaging",
        "messaging_channel": "whatsapp",
        "user_data": user_data,
        "custom_data": custom_data,
    });

    let mut payload = serde_json::json!({
        "data": [event_obj],
    });

    // Optional test_event_code for Meta Events Manager testing
    if let Some(code) = test_event_code.filter(|c| !c.trim().is_empty()) {
        payload["test_event_code"] = serde_json::Value::String(code.trim().to_string());
    }

    let url = format!(
        "https://graph.facebook.com/v21.0/{}/events?access_token={}",
        dataset_id,
        urlencoding_simple(token)
    );

    let payload_str = payload.to_string();

    log::info!(
        target: "zapfast::meta_capi",
        "Sending Meta conversion event '{}' for contact (has_clid={}, value=R${:.2})",
        event_name,
        has_clid,
        value_reais
    );

    let agent = crate::proxy::agent();
    let response = agent
        .post(&url)
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
                let message = if has_clid {
                    format!(
                        "Conversão '{}' (R$ {:.2}) enviada ao Meta com sucesso",
                        event_name, value_reais
                    )
                } else {
                    format!(
                        "Conversão '{}' (R$ {:.2}) enviada ao Meta (Plano B - Experimental)",
                        event_name, value_reais
                    )
                };
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
                    .unwrap_or("Erro desconhecido retornado pelo Meta");
                Err(format!(
                    "Erro Meta Graph API (HTTP {}): {}",
                    status, err_msg
                ))
            }
        }
        Err(err) => Err(format!("Falha na conexão com a API do Meta: {err}")),
    }
}

fn urlencoding_simple(s: &str) -> String {
    s.replace('&', "%26")
        .replace('+', "%2B")
        .replace(' ', "%20")
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
