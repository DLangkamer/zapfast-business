//! Click-to-WhatsApp (CTWA) ad attribution and discovery.
//!
//! Inspects incoming messages for Meta ad attribution metadata (`ContextInfo`
//! and `ExternalAdReplyInfo`), safely extracting ad IDs, headlines, body copies,
//! creative thumbnails, UTM parameters, and click identifiers (`ctwa_clid`).
//!
//! Adheres strictly to privacy rules: customer phone numbers are masked and
//! customer conversational text is never exposed in logs.

use super::*;
use crate::model::{ContactOrigin, CtwaDiscoveryLog};

/// Diagnostic discovery snapshot of ad attribution metadata arriving with a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdAttributionDiscovery {
    /// Masked chat identifier (e.g. `+55...1234`) for privacy-safe logs.
    pub chat_redacted: String,
    /// Whether this is the first message recorded for this chat.
    pub is_first_contact: bool,
    /// Meta Ad ID (`source_id`), if present.
    pub ad_id: Option<String>,
    /// Creative headline or title.
    pub title: Option<String>,
    /// Creative body text or snippet.
    pub body: Option<String>,
    /// Media type of the ad creative (image/video).
    pub media_type: Option<String>,
    /// Source / destination URL (e.g. fb.me link or Instagram post).
    pub source_url: Option<String>,
    /// Source type (e.g. "ad" / "facebook" / "instagram").
    pub source_type: Option<String>,
    /// Whether an embedded creative thumbnail was delivered.
    pub has_thumbnail: bool,
    /// Byte size of the embedded thumbnail JPEG/PNG.
    pub thumbnail_bytes: usize,
    /// URL to thumbnail image if provided as a link.
    pub thumbnail_url: Option<String>,
    /// Click-to-WhatsApp Click ID (`ctwa_clid`), if provided on the wire.
    pub ctwa_clid: Option<String>,
    /// Referral parameter (`ref`).
    pub ref_param: Option<String>,
    /// UTM Source parameter from context info.
    pub utm_source: Option<String>,
    /// UTM Campaign parameter from context info.
    pub utm_campaign: Option<String>,
    /// Conversion source identifier.
    pub conversion_source: Option<String>,
    /// Whether encrypted CTWA signals are present on the wire.
    pub has_ctwa_signals: bool,
    /// Whether CTWA payload bytes are present on the wire.
    pub has_ctwa_payload: bool,
    /// Optional greeting message body defined on the ad.
    pub greeting_message: Option<String>,
}

/// Redacts a phone number or JID to keep logs safe and privacy-compliant.
pub fn redact_chat_id(chat: &str) -> String {
    let digits: String = chat.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() >= 6 {
        let start = &digits[..2];
        let end = &digits[digits.len() - 4..];
        format!("+{}...{}", start, end)
    } else if !digits.is_empty() {
        format!("id#{}", digits)
    } else {
        "chat#redacted".to_string()
    }
}

/// Inspects a message for Click-to-WhatsApp (CTWA) metadata and logs diagnostic discovery.
pub fn inspect_ctwa(
    base: &wa::Message,
    chat: &str,
    is_first_contact: bool,
) -> Option<AdAttributionDiscovery> {
    let context = context_of(base)?;
    let ad_reply = context.external_ad_reply.as_option();
    let has_quoted_ad = context.quoted_ad.is_set();
    let has_utm = context.utm.is_set();
    let has_signals = context.ctwa_signals.is_some() || context.ctwa_payload.is_some();

    if ad_reply.is_none() && !has_quoted_ad && !has_utm && !has_signals {
        return None;
    }

    let chat_redacted = redact_chat_id(chat);

    let (
        ad_id,
        title,
        body,
        media_type,
        source_url,
        source_type,
        has_thumbnail,
        thumbnail_bytes,
        thumbnail_url,
        ctwa_clid,
        ref_param,
        greeting_message,
    ) = if let Some(ad) = ad_reply {
        let thumb = ad.thumbnail.as_deref().unwrap_or(&[]);
        let thumb_bytes = thumb.len();
        let has_thumb = thumb_bytes > 0 || ad.thumbnail_url.is_some();
        let m_type = ad.media_type.map(|m| format!("{m:?}"));
        (
            ad.source_id.clone(),
            ad.title.clone(),
            ad.body.clone(),
            m_type,
            ad.source_url.clone(),
            ad.source_type.clone(),
            has_thumb,
            thumb_bytes,
            ad.thumbnail_url.clone(),
            ad.ctwa_clid.clone(),
            ad.r#ref.clone(),
            ad.greeting_message_body.clone(),
        )
    } else {
        (
            None, None, None, None, None, None, false, 0, None, None, None, None,
        )
    };

    let (utm_source, utm_campaign) = if let Some(utm) = context.utm.as_option() {
        (utm.utm_source.clone(), utm.utm_campaign.clone())
    } else {
        (None, None)
    };

    let discovery = AdAttributionDiscovery {
        chat_redacted: chat_redacted.clone(),
        is_first_contact,
        ad_id,
        title,
        body,
        media_type,
        source_url,
        source_type,
        has_thumbnail,
        thumbnail_bytes,
        thumbnail_url,
        ctwa_clid,
        ref_param,
        utm_source,
        utm_campaign,
        conversion_source: context.conversion_source.clone(),
        has_ctwa_signals: context.ctwa_signals.is_some(),
        has_ctwa_payload: context.ctwa_payload.is_some(),
        greeting_message,
    };

    log::info!(
        target: "zapfast::ctwa",
        "CTWA discovery [chat={} first_contact={}]: ad_id={:?} title={:?} has_thumb={} ({} bytes) ctwa_clid_present={} ctwa_clid_len={} utm_source={:?} utm_campaign={:?} has_signals={}",
        discovery.chat_redacted,
        discovery.is_first_contact,
        discovery.ad_id,
        discovery.title,
        discovery.has_thumbnail,
        discovery.thumbnail_bytes,
        discovery.ctwa_clid.is_some(),
        discovery.ctwa_clid.as_ref().map(|s| s.len()).unwrap_or(0),
        discovery.utm_source,
        discovery.utm_campaign,
        discovery.has_ctwa_signals || discovery.has_ctwa_payload,
    );

    Some(discovery)
}

/// Inspects an incoming message, logs diagnostic CTWA discovery to the audit table,
/// and stores the first-touch contact origin in SQLite (never overwriting existing origin).
///
/// If explicit CTWA metadata is missing but this is the first contact, checks
/// Plan B text pattern attribution rules (e.g. ad default greeting message matching).
pub fn handle_incoming_ctwa(
    archive: &Archive,
    base: &wa::Message,
    chat: &str,
    timestamp: i64,
) -> Option<ContactOrigin> {
    let is_first_contact = !archive.chat_has_messages(chat).unwrap_or(false);

    if let Some(discovery) = inspect_ctwa(base, chat, is_first_contact) {
        // 1. Audit log entry (for real lead validation)
        let discovery_log = CtwaDiscoveryLog {
            id: 0,
            chat_masked: discovery.chat_redacted.clone(),
            is_first_contact,
            ad_id: discovery.ad_id.clone(),
            title: discovery.title.clone(),
            body: discovery.body.clone(),
            media_type: discovery.media_type.clone(),
            source_url: discovery.source_url.clone(),
            source_type: discovery.source_type.clone(),
            has_thumbnail: discovery.has_thumbnail,
            thumbnail_bytes: discovery.thumbnail_bytes,
            ctwa_clid: discovery.ctwa_clid.clone(),
            ref_param: discovery.ref_param.clone(),
            utm_source: discovery.utm_source.clone(),
            utm_campaign: discovery.utm_campaign.clone(),
            conversion_source: discovery.conversion_source.clone(),
            has_signals: discovery.has_ctwa_signals,
            has_payload: discovery.has_ctwa_payload,
            greeting_message: discovery.greeting_message.clone(),
            created_at: timestamp,
        };
        if let Err(error) = archive.log_ctwa_discovery(&discovery_log) {
            log::warn!("could not record CTWA discovery audit log: {error}");
        }

        // 2. Resolve ad mapping if registered
        let mut ad_name = String::new();
        let mut campaign_name = discovery.utm_campaign.clone().unwrap_or_default();
        let mut adset_name = String::new();

        if let Some(ad_id) = discovery.ad_id.as_deref() {
            if let Ok(Some(mapping)) = archive.ad_mapping(ad_id) {
                if !mapping.ad_name.is_empty() {
                    ad_name = mapping.ad_name;
                }
                if !mapping.campaign_name.is_empty() {
                    campaign_name = mapping.campaign_name;
                }
                if !mapping.adset_name.is_empty() {
                    adset_name = mapping.adset_name;
                }
            }
        }

        // If ad_name was not resolved from mapping table, fallback to creative title
        if ad_name.is_empty() {
            if let Some(title) = &discovery.title {
                ad_name = title.clone();
            } else if let Some(ad_id) = &discovery.ad_id {
                ad_name = format!("Anúncio {ad_id}");
            }
        }

        // Extract thumbnail bytes if present in the message
        let thumbnail = context_of(base)
            .and_then(|c| c.external_ad_reply.as_option())
            .and_then(|ad| ad.thumbnail.clone());

        let origin = ContactOrigin {
            chat_id: chat.to_string(),
            ad_id: discovery.ad_id,
            ad_name,
            campaign_name,
            adset_name,
            title: discovery.title.unwrap_or_default(),
            body: discovery.body.unwrap_or_default(),
            source_type: discovery.source_type.unwrap_or_default(),
            source_url: discovery.source_url.unwrap_or_default(),
            thumbnail,
            thumbnail_path: None,
            ctwa_clid: discovery.ctwa_clid,
            first_contact_at: timestamp,
            created_at: timestamp,
        };

        match archive.record_contact_origin(&origin) {
            Ok(true) => {
                log::info!(
                    target: "zapfast::ctwa",
                    "Stored new first-touch ad origin for contact {} (ad: {:?}, campaign: {:?})",
                    discovery.chat_redacted,
                    origin.ad_name,
                    origin.campaign_name,
                );
                Some(origin)
            }
            Ok(false) => {
                log::debug!(
                    target: "zapfast::ctwa",
                    "Contact {} already has a first-touch origin recorded; preserved existing",
                    discovery.chat_redacted,
                );
                archive.contact_origin(chat).ok().flatten()
            }
            Err(error) => {
                log::warn!("could not persist contact ad origin: {error}");
                None
            }
        }
    } else if is_first_contact {
        // Plan B: Text pattern / keyword matching for greeting messages configured in Meta Ads
        let text = base.text_content()?;
        let rule = archive.match_text_attribution_rule(text).ok().flatten()?;

        let chat_redacted = redact_chat_id(chat);
        log::info!(
            target: "zapfast::ctwa",
            "Plan B text match [chat={}]: rule #{} ({:?}) -> ad={:?} campaign={:?}",
            chat_redacted,
            rule.id,
            rule.pattern,
            rule.ad_name,
            rule.campaign_name,
        );

        let discovery_log = CtwaDiscoveryLog {
            id: 0,
            chat_masked: chat_redacted.clone(),
            is_first_contact: true,
            ad_id: rule.ad_id.clone(),
            title: Some(format!("Plano B: {}", rule.pattern)),
            body: Some(format!("Regra #{} ({})", rule.id, rule.match_type)),
            media_type: None,
            source_url: None,
            source_type: Some("plano_b_text".to_string()),
            has_thumbnail: false,
            thumbnail_bytes: 0,
            ctwa_clid: None,
            ref_param: None,
            utm_source: None,
            utm_campaign: Some(rule.campaign_name.clone()),
            conversion_source: Some("plano_b_text".to_string()),
            has_signals: false,
            has_payload: false,
            greeting_message: Some(text.to_string()),
            created_at: timestamp,
        };
        if let Err(error) = archive.log_ctwa_discovery(&discovery_log) {
            log::warn!("could not record Plan B CTWA discovery audit log: {error}");
        }

        let origin = ContactOrigin {
            chat_id: chat.to_string(),
            ad_id: rule.ad_id,
            ad_name: rule.ad_name,
            campaign_name: rule.campaign_name,
            adset_name: String::new(),
            title: format!("Plano B ({})", rule.match_type),
            body: format!("Padrão: {}", rule.pattern),
            source_type: "plano_b_text".to_string(),
            source_url: String::new(),
            thumbnail: None,
            thumbnail_path: None,
            ctwa_clid: None,
            first_contact_at: timestamp,
            created_at: timestamp,
        };

        match archive.record_contact_origin(&origin) {
            Ok(true) => {
                log::info!(
                    target: "zapfast::ctwa",
                    "Stored new first-touch Plan B text origin for contact {}",
                    chat_redacted,
                );
                Some(origin)
            }
            Ok(false) => archive.contact_origin(chat).ok().flatten(),
            Err(error) => {
                log::warn!("could not persist Plan B origin: {error}");
                None
            }
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AdCampaignMapping;

    #[test]
    fn redacts_sensitive_phone_numbers() {
        assert_eq!(redact_chat_id("5511999991234@s.whatsapp.net"), "+55...1234");
        assert_eq!(redact_chat_id("+15551234567"), "+15...4567");
        assert_eq!(redact_chat_id("group-123456"), "+12...3456");
    }

    #[test]
    fn returns_none_when_no_ad_metadata() {
        let msg = wa::Message::text("Olá, tudo bem?");
        let base = msg.get_base_message();
        assert_eq!(
            inspect_ctwa(base, "5511999991234@s.whatsapp.net", true),
            None
        );
    }

    #[test]
    fn extracts_full_ad_metadata() {
        let mut msg = wa::Message::default();
        let mut ext_text = wa::message::ExtendedTextMessage {
            text: Some("Olá! Vim pelo anúncio".into()),
            ..Default::default()
        };

        let mut context = wa::ContextInfo::default();
        let ad = wa::context_info::ExternalAdReplyInfo {
            source_id: Some("120208123456789".into()),
            title: Some("Super Promoção de Lançamento".into()),
            body: Some("Conheça o novo método com 50% de desconto".into()),
            source_url: Some("https://fb.me/ad123".into()),
            thumbnail: Some(vec![0xFF, 0xD8, 0xFF, 0xE0]), // dummy JPEG header
            ctwa_clid: Some("ctwa.clid.example.12345".into()),
            r#ref: Some("campanha_outubro".into()),
            ..Default::default()
        };

        context.external_ad_reply = MessageField::some(ad);
        ext_text.context_info = MessageField::some(context);
        msg.extended_text_message = MessageField::some(ext_text);

        let base = msg.get_base_message();
        let discovery = inspect_ctwa(base, "5511999991234@s.whatsapp.net", true).unwrap();

        assert_eq!(discovery.chat_redacted, "+55...1234");
        assert!(discovery.is_first_contact);
        assert_eq!(discovery.ad_id.as_deref(), Some("120208123456789"));
        assert_eq!(
            discovery.title.as_deref(),
            Some("Super Promoção de Lançamento")
        );
        assert_eq!(
            discovery.body.as_deref(),
            Some("Conheça o novo método com 50% de desconto")
        );
        assert_eq!(discovery.source_url.as_deref(), Some("https://fb.me/ad123"));
        assert!(discovery.has_thumbnail);
        assert_eq!(discovery.thumbnail_bytes, 4);
        assert_eq!(
            discovery.ctwa_clid.as_deref(),
            Some("ctwa.clid.example.12345")
        );
        assert_eq!(discovery.ref_param.as_deref(), Some("campanha_outubro"));
    }

    #[test]
    fn extracts_ad_metadata_when_ctwa_clid_is_missing() {
        let mut msg = wa::Message::default();
        let mut ext_text = wa::message::ExtendedTextMessage {
            text: Some("Olá! Quero mais informações".into()),
            ..Default::default()
        };

        let mut context = wa::ContextInfo::default();
        let ad = wa::context_info::ExternalAdReplyInfo {
            source_id: Some("987654321".into()),
            title: Some("Curso Avançado".into()),
            source_url: Some("https://instagram.com/p/abc".into()),
            // ctwa_clid is NOT set (simulating multi-device companion scenario where ctwa_clid is omitted)
            ..Default::default()
        };

        context.external_ad_reply = MessageField::some(ad);
        ext_text.context_info = MessageField::some(context);
        msg.extended_text_message = MessageField::some(ext_text);

        let base = msg.get_base_message();
        let discovery = inspect_ctwa(base, "5521988887777@s.whatsapp.net", true).unwrap();

        assert_eq!(discovery.ad_id.as_deref(), Some("987654321"));
        assert_eq!(discovery.title.as_deref(), Some("Curso Avançado"));
        assert_eq!(discovery.ctwa_clid, None);
    }

    #[test]
    fn persists_origin_and_never_overwrites_first_contact() {
        let archive = Archive::in_memory().unwrap();
        let chat = "5511999991234@s.whatsapp.net";

        // Register mapping for ad 111
        archive
            .save_ad_mapping(&AdCampaignMapping {
                ad_id: "111".into(),
                ad_name: "Criativo A".into(),
                campaign_name: "Campanha Black Friday".into(),
                adset_name: "Público Aberto".into(),
                source: "manual".into(),
                updated_at: 1000,
            })
            .unwrap();

        // 1. First ad message arrives from Ad 111
        let mut msg1 = wa::Message::default();
        let mut ext1 = wa::message::ExtendedTextMessage::default();
        let mut ctx1 = wa::ContextInfo::default();
        let ad1 = wa::context_info::ExternalAdReplyInfo {
            source_id: Some("111".into()),
            title: Some("Headline A".into()),
            body: Some("Texto do anúncio A".into()),
            thumbnail: Some(vec![1, 2, 3]),
            ctwa_clid: Some("clid_first".into()),
            ..Default::default()
        };
        ctx1.external_ad_reply = MessageField::some(ad1);
        ext1.context_info = MessageField::some(ctx1);
        msg1.extended_text_message = MessageField::some(ext1);

        let origin1 = handle_incoming_ctwa(&archive, msg1.get_base_message(), chat, 1000).unwrap();
        assert_eq!(origin1.ad_id.as_deref(), Some("111"));
        assert_eq!(origin1.ad_name, "Criativo A");
        assert_eq!(origin1.campaign_name, "Campanha Black Friday");
        assert_eq!(origin1.adset_name, "Público Aberto");
        assert_eq!(origin1.ctwa_clid.as_deref(), Some("clid_first"));
        assert_eq!(origin1.first_contact_at, 1000);

        // Verify stored in archive
        let stored = archive.contact_origin(chat).unwrap().unwrap();
        assert_eq!(stored.ad_id.as_deref(), Some("111"));
        assert_eq!(stored.first_contact_at, 1000);

        // 2. Later, a second ad message arrives from Ad 222 (e.g. user clicked another ad)
        let mut msg2 = wa::Message::default();
        let mut ext2 = wa::message::ExtendedTextMessage::default();
        let mut ctx2 = wa::ContextInfo::default();
        let ad2 = wa::context_info::ExternalAdReplyInfo {
            source_id: Some("222".into()),
            title: Some("Headline B".into()),
            ctwa_clid: Some("clid_second".into()),
            ..Default::default()
        };
        ctx2.external_ad_reply = MessageField::some(ad2);
        ext2.context_info = MessageField::some(ctx2);
        msg2.extended_text_message = MessageField::some(ext2);

        let origin2 = handle_incoming_ctwa(&archive, msg2.get_base_message(), chat, 2000).unwrap();
        // MUST NEVER OVERWRITE! Origin remains 111 and timestamp remains 1000!
        assert_eq!(origin2.ad_id.as_deref(), Some("111"));
        assert_eq!(origin2.ad_name, "Criativo A");
        assert_eq!(origin2.ctwa_clid.as_deref(), Some("clid_first"));
        assert_eq!(origin2.first_contact_at, 1000);

        let stored_after = archive.contact_origin(chat).unwrap().unwrap();
        assert_eq!(stored_after.ad_id.as_deref(), Some("111"));
        assert_eq!(stored_after.first_contact_at, 1000);

        // But both discovery logs were recorded for live audit & test validation!
        let logs = archive.latest_ctwa_discoveries(10).unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].ad_id.as_deref(), Some("222")); // latest log
        assert_eq!(logs[1].ad_id.as_deref(), Some("111")); // first log
        assert_eq!(logs[0].chat_masked, "+55...1234");
    }

    #[test]
    fn matches_plan_b_text_rules_when_ctwa_metadata_is_absent() {
        use crate::model::TextAttributionRule;

        let archive = Archive::in_memory().unwrap();
        let chat = "5511988889999@s.whatsapp.net";

        // Save a Plan B rule: when someone says "Quero saber mais sobre o Plano VIP", match Ad 999
        let rule = TextAttributionRule {
            id: 0,
            ad_id: Some("999".to_string()),
            ad_name: "Vídeo 01 - Oferta VIP".to_string(),
            campaign_name: "Campanha Lançamento VIP".to_string(),
            adset_name: String::new(),
            pattern: "Plano VIP".to_string(),
            match_type: "contains".to_string(),
            case_sensitive: false,
            active: true,
            created_at: 1000,
        };
        archive.save_text_attribution_rule(&rule).unwrap();

        // Lead sends standard text message with no external_ad_reply
        let msg = wa::Message::text("Olá! Quero saber mais sobre o Plano VIP por favor");
        let origin = handle_incoming_ctwa(&archive, msg.get_base_message(), chat, 1500).unwrap();

        assert_eq!(origin.ad_id.as_deref(), Some("999"));
        assert_eq!(origin.ad_name, "Vídeo 01 - Oferta VIP");
        assert_eq!(origin.campaign_name, "Campanha Lançamento VIP");
        assert_eq!(origin.source_type, "plano_b_text");
        assert_eq!(origin.ctwa_clid, None); // Absent, will be handled via Plan B experimental in CAPI

        // Verify stored in archive
        let stored = archive.contact_origin(chat).unwrap().unwrap();
        assert_eq!(stored.source_type, "plano_b_text");
        assert_eq!(stored.ad_name, "Vídeo 01 - Oferta VIP");
    }
}
