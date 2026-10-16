//! SQLite storage for ad origin attribution, campaign mappings, and discovery logs.

use std::collections::HashMap;

use rusqlite::{OptionalExtension, params};

use super::{Archive, Result};
use crate::model::{
    AdCampaignMapping, ContactOrigin, CtwaDiscoveryLog, MetaConversionLog, TextAttributionRule,
};

pub const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS contact_origins (
    chat_id TEXT PRIMARY KEY,
    ad_id TEXT,
    ad_name TEXT NOT NULL DEFAULT '',
    campaign_name TEXT NOT NULL DEFAULT '',
    adset_name TEXT NOT NULL DEFAULT '',
    title TEXT NOT NULL DEFAULT '',
    body TEXT NOT NULL DEFAULT '',
    source_type TEXT NOT NULL DEFAULT '',
    source_url TEXT NOT NULL DEFAULT '',
    thumbnail BLOB,
    thumbnail_path TEXT,
    ctwa_clid TEXT,
    first_contact_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_contact_origins_ad ON contact_origins (ad_id);
CREATE INDEX IF NOT EXISTS idx_contact_origins_campaign ON contact_origins (campaign_name);

CREATE TABLE IF NOT EXISTS ad_campaign_mappings (
    ad_id TEXT PRIMARY KEY,
    ad_name TEXT NOT NULL DEFAULT '',
    campaign_name TEXT NOT NULL DEFAULT '',
    adset_name TEXT NOT NULL DEFAULT '',
    source TEXT NOT NULL DEFAULT 'manual',
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS ctwa_discovery_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chat_masked TEXT NOT NULL,
    is_first_contact INTEGER NOT NULL,
    ad_id TEXT,
    title TEXT,
    body TEXT,
    media_type TEXT,
    source_url TEXT,
    source_type TEXT,
    has_thumbnail INTEGER NOT NULL DEFAULT 0,
    thumbnail_bytes INTEGER NOT NULL DEFAULT 0,
    ctwa_clid TEXT,
    ref_param TEXT,
    utm_source TEXT,
    utm_campaign TEXT,
    conversion_source TEXT,
    has_signals INTEGER NOT NULL DEFAULT 0,
    has_payload INTEGER NOT NULL DEFAULT 0,
    greeting_message TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ctwa_logs_created ON ctwa_discovery_logs (created_at DESC);

CREATE TABLE IF NOT EXISTS text_attribution_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ad_id TEXT,
    ad_name TEXT NOT NULL DEFAULT '',
    campaign_name TEXT NOT NULL DEFAULT '',
    adset_name TEXT NOT NULL DEFAULT '',
    pattern TEXT NOT NULL,
    match_type TEXT NOT NULL DEFAULT 'contains',
    case_sensitive INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS meta_conversion_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chat_id TEXT NOT NULL,
    event_name TEXT NOT NULL DEFAULT 'Purchase',
    value_cents INTEGER NOT NULL DEFAULT 0,
    currency TEXT NOT NULL DEFAULT 'BRL',
    ctwa_clid TEXT,
    phone_masked TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'sent',
    response_json TEXT NOT NULL DEFAULT '',
    error_message TEXT NOT NULL DEFAULT '',
    sent_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_meta_conversion_logs_chat ON meta_conversion_logs (chat_id, sent_at DESC);
";

impl Archive {
    /// Records the first-touch ad origin for a contact.
    ///
    /// Never overwrites an existing contact origin (uses `ON CONFLICT DO NOTHING`).
    /// Returns `true` if a new origin was stored, or `false` if one was already present.
    pub fn record_contact_origin(&self, origin: &ContactOrigin) -> Result<bool> {
        let rows = self.connection.execute(
            "INSERT INTO contact_origins (
                chat_id, ad_id, ad_name, campaign_name, adset_name,
                title, body, source_type, source_url, thumbnail,
                thumbnail_path, ctwa_clid, first_contact_at, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(chat_id) DO NOTHING",
            params![
                origin.chat_id,
                origin.ad_id,
                origin.ad_name,
                origin.campaign_name,
                origin.adset_name,
                origin.title,
                origin.body,
                origin.source_type,
                origin.source_url,
                origin.thumbnail,
                origin.thumbnail_path,
                origin.ctwa_clid,
                origin.first_contact_at,
                origin.created_at,
            ],
        )?;
        Ok(rows > 0)
    }

    /// Fetches the ad origin for a specific contact.
    pub fn contact_origin(&self, chat_id: &str) -> Result<Option<ContactOrigin>> {
        self.connection
            .query_row(
                "SELECT chat_id, ad_id, ad_name, campaign_name, adset_name,
                        title, body, source_type, source_url, thumbnail,
                        thumbnail_path, ctwa_clid, first_contact_at, created_at
                 FROM contact_origins
                 WHERE chat_id = ?1",
                params![chat_id],
                |row| {
                    Ok(ContactOrigin {
                        chat_id: row.get(0)?,
                        ad_id: row.get(1)?,
                        ad_name: row.get(2)?,
                        campaign_name: row.get(3)?,
                        adset_name: row.get(4)?,
                        title: row.get(5)?,
                        body: row.get(6)?,
                        source_type: row.get(7)?,
                        source_url: row.get(8)?,
                        thumbnail: row.get(9)?,
                        thumbnail_path: row.get(10)?,
                        ctwa_clid: row.get(11)?,
                        first_contact_at: row.get(12)?,
                        created_at: row.get(13)?,
                    })
                },
            )
            .optional()
    }

    /// Loads all known contact origins into a lookup table keyed by chat ID.
    pub fn list_contact_origins(&self) -> Result<HashMap<String, ContactOrigin>> {
        let mut statement = self.connection.prepare(
            "SELECT chat_id, ad_id, ad_name, campaign_name, adset_name,
                    title, body, source_type, source_url, thumbnail,
                    thumbnail_path, ctwa_clid, first_contact_at, created_at
             FROM contact_origins",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ContactOrigin {
                chat_id: row.get(0)?,
                ad_id: row.get(1)?,
                ad_name: row.get(2)?,
                campaign_name: row.get(3)?,
                adset_name: row.get(4)?,
                title: row.get(5)?,
                body: row.get(6)?,
                source_type: row.get(7)?,
                source_url: row.get(8)?,
                thumbnail: row.get(9)?,
                thumbnail_path: row.get(10)?,
                ctwa_clid: row.get(11)?,
                first_contact_at: row.get(12)?,
                created_at: row.get(13)?,
            })
        })?;
        let mut origins = HashMap::new();
        for row in rows {
            let origin = row?;
            origins.insert(origin.chat_id.clone(), origin);
        }
        Ok(origins)
    }

    /// Gets a registered Ad ID mapping (Ad Name, Campaign, Ad Set).
    pub fn ad_mapping(&self, ad_id: &str) -> Result<Option<AdCampaignMapping>> {
        self.connection
            .query_row(
                "SELECT ad_id, ad_name, campaign_name, adset_name, source, updated_at
                 FROM ad_campaign_mappings
                 WHERE ad_id = ?1",
                params![ad_id],
                |row| {
                    Ok(AdCampaignMapping {
                        ad_id: row.get(0)?,
                        ad_name: row.get(1)?,
                        campaign_name: row.get(2)?,
                        adset_name: row.get(3)?,
                        source: row.get(4)?,
                        updated_at: row.get(5)?,
                    })
                },
            )
            .optional()
    }

    /// Creates or updates an ad ID mapping (from manual entry or Meta API query).
    pub fn save_ad_mapping(&self, mapping: &AdCampaignMapping) -> Result<()> {
        self.connection.execute(
            "INSERT INTO ad_campaign_mappings (
                ad_id, ad_name, campaign_name, adset_name, source, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(ad_id) DO UPDATE SET
                ad_name = excluded.ad_name,
                campaign_name = excluded.campaign_name,
                adset_name = excluded.adset_name,
                source = excluded.source,
                updated_at = excluded.updated_at",
            params![
                mapping.ad_id,
                mapping.ad_name,
                mapping.campaign_name,
                mapping.adset_name,
                mapping.source,
                mapping.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Lists all registered ad ID mappings.
    pub fn list_ad_mappings(&self) -> Result<Vec<AdCampaignMapping>> {
        let mut statement = self.connection.prepare(
            "SELECT ad_id, ad_name, campaign_name, adset_name, source, updated_at
             FROM ad_campaign_mappings
             ORDER BY updated_at DESC, ad_name ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(AdCampaignMapping {
                ad_id: row.get(0)?,
                ad_name: row.get(1)?,
                campaign_name: row.get(2)?,
                adset_name: row.get(3)?,
                source: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    /// Deletes an ad mapping.
    pub fn delete_ad_mapping(&self, ad_id: &str) -> Result<bool> {
        let rows = self.connection.execute(
            "DELETE FROM ad_campaign_mappings WHERE ad_id = ?1",
            params![ad_id],
        )?;
        Ok(rows > 0)
    }

    /// Records an audit log entry for a captured CTWA payload (for real lead validation).
    pub fn log_ctwa_discovery(&self, log: &CtwaDiscoveryLog) -> Result<()> {
        self.connection.execute(
            "INSERT INTO ctwa_discovery_logs (
                chat_masked, is_first_contact, ad_id, title, body,
                media_type, source_url, source_type, has_thumbnail,
                thumbnail_bytes, ctwa_clid, ref_param, utm_source,
                utm_campaign, conversion_source, has_signals, has_payload,
                greeting_message, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                log.chat_masked,
                log.is_first_contact as i64,
                log.ad_id,
                log.title,
                log.body,
                log.media_type,
                log.source_url,
                log.source_type,
                log.has_thumbnail as i64,
                log.thumbnail_bytes as i64,
                log.ctwa_clid,
                log.ref_param,
                log.utm_source,
                log.utm_campaign,
                log.conversion_source,
                log.has_signals as i64,
                log.has_payload as i64,
                log.greeting_message,
                log.created_at,
            ],
        )?;
        Ok(())
    }

    /// Returns the most recent CTWA discovery logs for live inspection and test validation.
    pub fn latest_ctwa_discoveries(&self, limit: usize) -> Result<Vec<CtwaDiscoveryLog>> {
        let mut statement = self.connection.prepare(
            "SELECT id, chat_masked, is_first_contact, ad_id, title, body,
                    media_type, source_url, source_type, has_thumbnail,
                    thumbnail_bytes, ctwa_clid, ref_param, utm_source,
                    utm_campaign, conversion_source, has_signals, has_payload,
                    greeting_message, created_at
             FROM ctwa_discovery_logs
             ORDER BY id DESC
             LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit as i64], |row| {
            Ok(CtwaDiscoveryLog {
                id: row.get(0)?,
                chat_masked: row.get(1)?,
                is_first_contact: row.get::<_, i64>(2)? != 0,
                ad_id: row.get(3)?,
                title: row.get(4)?,
                body: row.get(5)?,
                media_type: row.get(6)?,
                source_url: row.get(7)?,
                source_type: row.get(8)?,
                has_thumbnail: row.get::<_, i64>(9)? != 0,
                thumbnail_bytes: row.get::<_, i64>(10)? as usize,
                ctwa_clid: row.get(11)?,
                ref_param: row.get(12)?,
                utm_source: row.get(13)?,
                utm_campaign: row.get(14)?,
                conversion_source: row.get(15)?,
                has_signals: row.get::<_, i64>(16)? != 0,
                has_payload: row.get::<_, i64>(17)? != 0,
                greeting_message: row.get(18)?,
                created_at: row.get(19)?,
            })
        })?;
        rows.collect()
    }

    /// Saves or creates a Plan B text attribution rule.
    pub fn save_text_attribution_rule(&self, rule: &TextAttributionRule) -> Result<i64> {
        if rule.id > 0 {
            self.connection.execute(
                "UPDATE text_attribution_rules
                 SET ad_id = ?1, ad_name = ?2, campaign_name = ?3, adset_name = ?4,
                     pattern = ?5, match_type = ?6, case_sensitive = ?7, active = ?8
                 WHERE id = ?9",
                params![
                    rule.ad_id,
                    rule.ad_name,
                    rule.campaign_name,
                    rule.adset_name,
                    rule.pattern,
                    rule.match_type,
                    if rule.case_sensitive { 1 } else { 0 },
                    if rule.active { 1 } else { 0 },
                    rule.id,
                ],
            )?;
            Ok(rule.id)
        } else {
            self.connection.execute(
                "INSERT INTO text_attribution_rules (
                    ad_id, ad_name, campaign_name, adset_name, pattern,
                    match_type, case_sensitive, active, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    rule.ad_id,
                    rule.ad_name,
                    rule.campaign_name,
                    rule.adset_name,
                    rule.pattern,
                    rule.match_type,
                    if rule.case_sensitive { 1 } else { 0 },
                    if rule.active { 1 } else { 0 },
                    rule.created_at,
                ],
            )?;
            Ok(self.connection.last_insert_rowid())
        }
    }

    /// Lists all Plan B text attribution rules.
    pub fn list_text_attribution_rules(&self) -> Result<Vec<TextAttributionRule>> {
        let mut statement = self.connection.prepare(
            "SELECT id, ad_id, ad_name, campaign_name, adset_name,
                    pattern, match_type, case_sensitive, active, created_at
             FROM text_attribution_rules
             ORDER BY id ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(TextAttributionRule {
                id: row.get(0)?,
                ad_id: row.get(1)?,
                ad_name: row.get(2)?,
                campaign_name: row.get(3)?,
                adset_name: row.get(4)?,
                pattern: row.get(5)?,
                match_type: row.get(6)?,
                case_sensitive: row.get::<_, i64>(7)? != 0,
                active: row.get::<_, i64>(8)? != 0,
                created_at: row.get(9)?,
            })
        })?;
        rows.collect()
    }

    /// Deletes a text attribution rule by id.
    pub fn delete_text_attribution_rule(&self, id: i64) -> Result<bool> {
        let rows = self.connection.execute(
            "DELETE FROM text_attribution_rules WHERE id = ?1",
            params![id],
        )?;
        Ok(rows > 0)
    }

    /// Matches the first active text rule that matches the message text.
    pub fn match_text_attribution_rule(&self, text: &str) -> Result<Option<TextAttributionRule>> {
        let rules = self.list_text_attribution_rules()?;
        for rule in rules {
            if rule.matches(text) {
                return Ok(Some(rule));
            }
        }
        Ok(None)
    }

    /// Records an audit log entry for a Meta Conversions API event submission.
    pub fn log_meta_conversion(&self, log: &MetaConversionLog) -> Result<i64> {
        self.connection.execute(
            "INSERT INTO meta_conversion_logs (
                chat_id, event_name, value_cents, currency, ctwa_clid,
                phone_masked, status, response_json, error_message, sent_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                log.chat_id,
                log.event_name,
                log.value_cents,
                log.currency,
                log.ctwa_clid,
                log.phone_masked,
                log.status,
                log.response_json,
                log.error_message,
                log.sent_at,
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    /// Returns the latest conversion log for a chat.
    pub fn latest_conversion_for_chat(&self, chat_id: &str) -> Result<Option<MetaConversionLog>> {
        self.connection
            .query_row(
                "SELECT id, chat_id, event_name, value_cents, currency, ctwa_clid,
                        phone_masked, status, response_json, error_message, sent_at
                 FROM meta_conversion_logs
                 WHERE chat_id = ?1
                 ORDER BY sent_at DESC
                 LIMIT 1",
                params![chat_id],
                |row| {
                    Ok(MetaConversionLog {
                        id: row.get(0)?,
                        chat_id: row.get(1)?,
                        event_name: row.get(2)?,
                        value_cents: row.get(3)?,
                        currency: row.get(4)?,
                        ctwa_clid: row.get(5)?,
                        phone_masked: row.get(6)?,
                        status: row.get(7)?,
                        response_json: row.get(8)?,
                        error_message: row.get(9)?,
                        sent_at: row.get(10)?,
                    })
                },
            )
            .optional()
    }

    /// Lists recent conversion logs across all chats.
    pub fn list_meta_conversions(&self, limit: usize) -> Result<Vec<MetaConversionLog>> {
        let mut statement = self.connection.prepare(
            "SELECT id, chat_id, event_name, value_cents, currency, ctwa_clid,
                    phone_masked, status, response_json, error_message, sent_at
             FROM meta_conversion_logs
             ORDER BY sent_at DESC
             LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit as i64], |row| {
            Ok(MetaConversionLog {
                id: row.get(0)?,
                chat_id: row.get(1)?,
                event_name: row.get(2)?,
                value_cents: row.get(3)?,
                currency: row.get(4)?,
                ctwa_clid: row.get(5)?,
                phone_masked: row.get(6)?,
                status: row.get(7)?,
                response_json: row.get(8)?,
                error_message: row.get(9)?,
                sent_at: row.get(10)?,
            })
        })?;
        rows.collect()
    }
}
