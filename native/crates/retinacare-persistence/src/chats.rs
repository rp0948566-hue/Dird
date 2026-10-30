//! Phase 5: private conversations in the encrypted database (schema v5), plus the cloud outbox.
//!
//! SQLCipher is the source of truth. A conversation is `research` (bound to the analysis cascade; may
//! reference a patient/examination) or `chat` (general assistant). Only a `chat` conversation with no
//! patient link and no attachments may be marked for cloud sync, and only then are its messages queued
//! in `chat_outbox`; the rule is enforced here, not by the caller. Message ids are client-generated
//! UUIDs, so a repeated append (retry, double click, replayed sync) is a no-op, never a duplicate.

use crate::{SqlCipherStore, StoreError};
use rand::{RngCore, rngs::OsRng};
use rusqlite::{OptionalExtension, params};

const MAX_TITLE: usize = 200;
const MAX_CONTENT: usize = 100_000;
const MAX_CARD_JSON: usize = 64 * 1024;
/// SQLite treats a negative LIMIT as no limit: every saved conversation is listed.
const MAX_LIST: i64 = -1;

pub(crate) const V5_SQL: &str = "
CREATE TABLE IF NOT EXISTS conversations (
  id INTEGER PRIMARY KEY,
  conv_uuid TEXT NOT NULL UNIQUE,
  title TEXT NOT NULL,
  mode TEXT NOT NULL CHECK (mode IN ('research', 'chat')),
  patient_id INTEGER REFERENCES patients(id) ON DELETE CASCADE,
  examination_id INTEGER REFERENCES examinations(id) ON DELETE SET NULL,
  sync_enabled INTEGER NOT NULL DEFAULT 0 CHECK (sync_enabled IN (0, 1)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);
CREATE INDEX IF NOT EXISTS conversations_updated_idx ON conversations (updated_at DESC);
CREATE TABLE IF NOT EXISTS conversation_messages (
  id INTEGER PRIMARY KEY,
  conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  client_msg_id TEXT NOT NULL UNIQUE,
  seq INTEGER NOT NULL,
  role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
  kind TEXT NOT NULL CHECK (kind IN ('text', 'analysis', 'error')),
  status TEXT NOT NULL,
  content TEXT NOT NULL,
  card_json TEXT,
  provider TEXT,
  attachment_count INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  UNIQUE (conversation_id, seq)
);
CREATE TABLE IF NOT EXISTS chat_outbox (
  id INTEGER PRIMARY KEY,
  conv_uuid TEXT NOT NULL,
  client_msg_id TEXT NOT NULL UNIQUE,
  attempts INTEGER NOT NULL DEFAULT 0,
  last_error TEXT,
  synced_at TEXT,
  created_at TEXT NOT NULL
);
";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversationMode {
    Research,
    Chat,
}

impl ConversationMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Research => "research",
            Self::Chat => "chat",
        }
    }
    fn parse(value: &str) -> Self {
        if value == "research" {
            Self::Research
        } else {
            Self::Chat
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConversationSummary {
    pub conv_uuid: String,
    pub title: String,
    pub mode: ConversationMode,
    pub patient_id: Option<i64>,
    pub examination_id: Option<i64>,
    pub sync_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: i64,
    pub preview: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StoredMessage {
    pub client_msg_id: String,
    pub seq: i64,
    pub role: String,
    pub kind: String,
    pub status: String,
    pub content: String,
    pub card_json: Option<String>,
    pub provider: Option<String>,
    pub attachment_count: i64,
    pub created_at: String,
}

/// One message to append. `client_msg_id` must be a UUID made by `new_uuid`.
#[derive(Clone, Debug, PartialEq)]
pub struct MessageInput {
    pub client_msg_id: String,
    pub role: String,
    pub kind: String,
    pub status: String,
    pub content: String,
    pub card_json: Option<String>,
    pub provider: Option<String>,
    pub attachment_count: i64,
}

/// One queued cloud write: a message of a sync-enabled general conversation.
#[derive(Clone, Debug, PartialEq)]
pub struct OutboxItem {
    pub outbox_id: i64,
    pub conv_uuid: String,
    pub conversation_title: String,
    pub message: StoredMessage,
    pub attempts: i64,
}

/// Random RFC 4122 version-4 UUID (the format the cloud schema uses).
pub fn new_uuid() -> String {
    let mut b = [0_u8; 16];
    OsRng.fill_bytes(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

pub fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && value.char_indices().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit() && !c.is_ascii_uppercase(),
        })
}

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%SZ', 'now')";

impl SqlCipherStore {
    /// Create a conversation. A research conversation may name a patient/examination that must exist
    /// and belong together; a chat conversation never carries a patient.
    pub fn create_conversation(
        &self,
        title: &str,
        mode: ConversationMode,
        patient_id: Option<i64>,
        examination_id: Option<i64>,
    ) -> Result<String, StoreError> {
        let title = title.trim();
        if title.is_empty() || title.chars().count() > MAX_TITLE {
            return Err(StoreError::InvalidInput);
        }
        if mode == ConversationMode::Chat && (patient_id.is_some() || examination_id.is_some()) {
            return Err(StoreError::InvalidInput);
        }
        if let Some(exam) = examination_id {
            let owner: Option<i64> = self
                .connection
                .query_row(
                    "SELECT patient_id FROM examinations WHERE id = ?1",
                    params![exam],
                    |r| r.get(0),
                )
                .optional()?;
            if owner.is_none() || owner != patient_id {
                return Err(StoreError::InvalidInput);
            }
        }
        let uuid = new_uuid();
        self.connection.execute(
            &format!("INSERT INTO conversations (conv_uuid, title, mode, patient_id, examination_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, {NOW}, {NOW})"),
            params![uuid, title, mode.as_str(), patient_id, examination_id],
        )?;
        Ok(uuid)
    }

    fn conversation_row(&self, conv_uuid: &str) -> Result<(i64, String, bool, bool), StoreError> {
        self.connection
            .query_row(
                "SELECT id, mode, patient_id IS NULL AND examination_id IS NULL, sync_enabled FROM conversations WHERE conv_uuid = ?1 AND deleted_at IS NULL",
                params![conv_uuid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get::<_, i64>(3)? == 1)),
            )
            .optional()?
            .ok_or(StoreError::InvalidInput)
    }

    /// Append a message. Returns `false` (and changes nothing) when that `client_msg_id` already exists.
    pub fn append_message(
        &self,
        conv_uuid: &str,
        input: &MessageInput,
    ) -> Result<bool, StoreError> {
        if !is_uuid(&input.client_msg_id)
            || !matches!(input.role.as_str(), "user" | "assistant" | "system")
            || !matches!(input.kind.as_str(), "text" | "analysis" | "error")
            || input.status.is_empty()
            || input.status.len() > 32
            || input.content.chars().count() > MAX_CONTENT
            || input
                .card_json
                .as_ref()
                .is_some_and(|c| c.len() > MAX_CARD_JSON)
            || input.provider.as_ref().is_some_and(|p| p.len() > 200)
            || !(0..=32).contains(&input.attachment_count)
        {
            return Err(StoreError::InvalidInput);
        }
        let (conv_id, mode, unlinked, sync) = self.conversation_row(conv_uuid)?;
        let transaction = self.connection.unchecked_transaction()?;
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM conversation_messages WHERE client_msg_id = ?1)",
            params![input.client_msg_id],
            |r| r.get(0),
        )?;
        if exists {
            return Ok(false);
        }
        let seq: i64 = transaction.query_row(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM conversation_messages WHERE conversation_id = ?1",
            params![conv_id],
            |r| r.get(0),
        )?;
        transaction.execute(
            &format!("INSERT INTO conversation_messages (conversation_id, client_msg_id, seq, role, kind, status, content, card_json, provider, attachment_count, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, {NOW})"),
            params![
                conv_id,
                input.client_msg_id,
                seq,
                input.role,
                input.kind,
                input.status,
                input.content,
                input.card_json,
                input.provider,
                input.attachment_count
            ],
        )?;
        transaction.execute(
            &format!("UPDATE conversations SET updated_at = {NOW} WHERE id = ?1"),
            params![conv_id],
        )?;
        // An attachment turns the conversation local-only for good.
        if input.attachment_count > 0 || input.kind == "analysis" {
            transaction.execute(
                "UPDATE conversations SET sync_enabled = 0 WHERE id = ?1",
                params![conv_id],
            )?;
            transaction.execute(
                "DELETE FROM chat_outbox WHERE conv_uuid = ?1 AND synced_at IS NULL",
                params![conv_uuid],
            )?;
        } else if sync && unlinked && mode == "chat" {
            transaction.execute(
                &format!("INSERT OR IGNORE INTO chat_outbox (conv_uuid, client_msg_id, created_at) VALUES (?1, ?2, {NOW})"),
                params![conv_uuid, input.client_msg_id],
            )?;
        }
        transaction.commit()?;
        Ok(true)
    }

    /// Final text/status of a streamed or failed assistant message. Only that message changes.
    pub fn update_message(
        &self,
        client_msg_id: &str,
        content: &str,
        status: &str,
        card_json: Option<&str>,
    ) -> Result<bool, StoreError> {
        if content.chars().count() > MAX_CONTENT
            || status.is_empty()
            || status.len() > 32
            || card_json.is_some_and(|c| c.len() > MAX_CARD_JSON)
        {
            return Err(StoreError::InvalidInput);
        }
        Ok(self.connection.execute(
            "UPDATE conversation_messages SET content = ?2, status = ?3, card_json = COALESCE(?4, card_json) WHERE client_msg_id = ?1",
            params![client_msg_id, content, status, card_json],
        )? > 0)
    }

    /// Newest first, soft-deleted rows hidden. `query` filters on title and message text (case-insensitive).
    pub fn list_conversations(&self, query: &str) -> Result<Vec<ConversationSummary>, StoreError> {
        let pattern = format!(
            "%{}%",
            query
                .trim()
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        let mut statement = self.connection.prepare(
            "SELECT c.conv_uuid, c.title, c.mode, c.patient_id, c.examination_id, c.sync_enabled, c.created_at, c.updated_at,
                    (SELECT count(*) FROM conversation_messages m WHERE m.conversation_id = c.id),
                    COALESCE((SELECT substr(m.content, 1, 160) FROM conversation_messages m WHERE m.conversation_id = c.id AND m.role = 'user' ORDER BY m.seq LIMIT 1), '')
             FROM conversations c
             WHERE c.deleted_at IS NULL AND (?1 = '%%' OR c.title LIKE ?1 ESCAPE '\\'
                   OR EXISTS (SELECT 1 FROM conversation_messages m WHERE m.conversation_id = c.id AND m.content LIKE ?1 ESCAPE '\\'))
             ORDER BY c.updated_at DESC, c.id DESC LIMIT ?2",
        )?;
        let rows = statement.query_map(params![pattern, MAX_LIST], |r| {
            Ok(ConversationSummary {
                conv_uuid: r.get(0)?,
                title: r.get(1)?,
                mode: ConversationMode::parse(&r.get::<_, String>(2)?),
                patient_id: r.get(3)?,
                examination_id: r.get(4)?,
                sync_enabled: r.get::<_, i64>(5)? == 1,
                created_at: r.get(6)?,
                updated_at: r.get(7)?,
                message_count: r.get(8)?,
                preview: r.get(9)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn load_messages(&self, conv_uuid: &str) -> Result<Vec<StoredMessage>, StoreError> {
        let (conv_id, ..) = self.conversation_row(conv_uuid)?;
        let mut statement = self.connection.prepare(
            "SELECT client_msg_id, seq, role, kind, status, content, card_json, provider, attachment_count, created_at FROM conversation_messages WHERE conversation_id = ?1 ORDER BY seq",
        )?;
        let rows = statement.query_map(params![conv_id], read_message)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn rename_conversation(&self, conv_uuid: &str, title: &str) -> Result<bool, StoreError> {
        let title = title.trim();
        if title.is_empty() || title.chars().count() > MAX_TITLE {
            return Err(StoreError::InvalidInput);
        }
        Ok(self.connection.execute(
            &format!("UPDATE conversations SET title = ?2, updated_at = {NOW} WHERE conv_uuid = ?1 AND deleted_at IS NULL"),
            params![conv_uuid, title],
        )? > 0)
    }

    /// Hide a conversation and drop its unsent outbox rows. Messages stay until the patient is deleted.
    pub fn delete_conversation(&self, conv_uuid: &str) -> Result<bool, StoreError> {
        let transaction = self.connection.unchecked_transaction()?;
        let changed = transaction.execute(
            &format!("UPDATE conversations SET deleted_at = {NOW}, sync_enabled = 0 WHERE conv_uuid = ?1 AND deleted_at IS NULL"),
            params![conv_uuid],
        )?;
        transaction.execute(
            "DELETE FROM chat_outbox WHERE conv_uuid = ?1 AND synced_at IS NULL",
            params![conv_uuid],
        )?;
        transaction.commit()?;
        Ok(changed > 0)
    }

    /// Permanently remove every conversation with its messages and unsent outbox rows. Patients,
    /// examinations, masks and reports are separate tables and are not touched.
    pub fn delete_all_conversations(&self) -> Result<usize, StoreError> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute("DELETE FROM conversation_messages", [])?;
        let removed = transaction.execute("DELETE FROM conversations", [])?;
        transaction.execute("DELETE FROM chat_outbox", [])?;
        transaction.commit()?;
        Ok(removed)
    }

    /// Turn cloud sync on or off. Refused for research conversations, patient-linked ones and any
    /// conversation that ever had an attachment or analysis card. Enabling queues the existing messages.
    pub fn set_conversation_sync(&self, conv_uuid: &str, enabled: bool) -> Result<(), StoreError> {
        let (conv_id, mode, unlinked, _) = self.conversation_row(conv_uuid)?;
        let transaction = self.connection.unchecked_transaction()?;
        if !enabled {
            transaction.execute(
                "UPDATE conversations SET sync_enabled = 0 WHERE id = ?1",
                params![conv_id],
            )?;
            transaction.execute(
                "DELETE FROM chat_outbox WHERE conv_uuid = ?1 AND synced_at IS NULL",
                params![conv_uuid],
            )?;
            transaction.commit()?;
            return Ok(());
        }
        let sensitive: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM conversation_messages WHERE conversation_id = ?1 AND (attachment_count > 0 OR kind = 'analysis'))",
            params![conv_id],
            |r| r.get(0),
        )?;
        if mode != "chat" || !unlinked || sensitive {
            return Err(StoreError::InvalidInput);
        }
        transaction.execute(
            "UPDATE conversations SET sync_enabled = 1 WHERE id = ?1",
            params![conv_id],
        )?;
        transaction.execute(
            &format!("INSERT OR IGNORE INTO chat_outbox (conv_uuid, client_msg_id, created_at) SELECT ?1, client_msg_id, {NOW} FROM conversation_messages WHERE conversation_id = ?2 ORDER BY seq"),
            params![conv_uuid, conv_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Bring a conversation of the signed-in cloud account onto this device (general chats only).
    /// Existing messages (same `client_msg_id`) are skipped, so a repeated pull never duplicates; the
    /// conversation is created with the remote id and marked synced, and nothing is queued for upload.
    pub fn import_remote_conversation(
        &self,
        conv_uuid: &str,
        title: &str,
        messages: &[MessageInput],
    ) -> Result<usize, StoreError> {
        let title = title.trim();
        if !is_uuid(conv_uuid) || title.is_empty() || title.chars().count() > MAX_TITLE {
            return Err(StoreError::InvalidInput);
        }
        if messages
            .iter()
            .any(|m| !is_uuid(&m.client_msg_id) || m.attachment_count != 0 || m.kind != "text")
        {
            return Err(StoreError::InvalidInput);
        }
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            &format!("INSERT OR IGNORE INTO conversations (conv_uuid, title, mode, sync_enabled, created_at, updated_at) VALUES (?1, ?2, 'chat', 1, {NOW}, {NOW})"),
            params![conv_uuid, title],
        )?;
        let (conv_id, deleted): (i64, Option<String>) = transaction.query_row(
            "SELECT id, deleted_at FROM conversations WHERE conv_uuid = ?1",
            params![conv_uuid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if deleted.is_some() {
            return Ok(0); // deleted here: a pull must not resurrect it
        }
        let mut added = 0usize;
        for m in messages {
            if m.content.chars().count() > MAX_CONTENT
                || !matches!(m.role.as_str(), "user" | "assistant" | "system")
            {
                return Err(StoreError::InvalidInput);
            }
            let exists: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM conversation_messages WHERE client_msg_id = ?1)",
                params![m.client_msg_id],
                |r| r.get(0),
            )?;
            if exists {
                continue;
            }
            let seq: i64 = transaction.query_row(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM conversation_messages WHERE conversation_id = ?1",
                params![conv_id],
                |r| r.get(0),
            )?;
            transaction.execute(
                &format!("INSERT INTO conversation_messages (conversation_id, client_msg_id, seq, role, kind, status, content, card_json, provider, attachment_count, created_at) VALUES (?1, ?2, ?3, ?4, 'text', 'completed', ?5, NULL, NULL, 0, {NOW})"),
                params![conv_id, m.client_msg_id, seq, m.role, m.content],
            )?;
            transaction.execute(
                &format!("INSERT OR IGNORE INTO chat_outbox (conv_uuid, client_msg_id, synced_at, created_at) VALUES (?1, ?2, {NOW}, {NOW})"),
                params![conv_uuid, m.client_msg_id],
            )?;
            added += 1;
        }
        transaction.commit()?;
        Ok(added)
    }

    /// Whether a conversation exists locally (soft-deleted ones count: they must not be re-imported).
    pub fn conversation_known(&self, conv_uuid: &str) -> Result<bool, StoreError> {
        Ok(self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM conversations WHERE conv_uuid = ?1)",
            params![conv_uuid],
            |r| r.get(0),
        )?)
    }

    /// Oldest unsent first. Rows whose conversation stopped being eligible are not returned.
    pub fn outbox_pending(&self, limit: i64) -> Result<Vec<OutboxItem>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT o.id, o.conv_uuid, c.title, o.attempts,
                    m.client_msg_id, m.seq, m.role, m.kind, m.status, m.content, m.card_json, m.provider, m.attachment_count, m.created_at
             FROM chat_outbox o
             JOIN conversations c ON c.conv_uuid = o.conv_uuid
             JOIN conversation_messages m ON m.client_msg_id = o.client_msg_id
             WHERE o.synced_at IS NULL AND c.sync_enabled = 1 AND c.deleted_at IS NULL AND c.mode = 'chat'
               AND c.patient_id IS NULL AND c.examination_id IS NULL AND m.attachment_count = 0 AND m.kind = 'text'
               AND m.status = 'completed'
             ORDER BY o.id LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit.clamp(1, 200)], |r| {
            Ok(OutboxItem {
                outbox_id: r.get(0)?,
                conv_uuid: r.get(1)?,
                conversation_title: r.get(2)?,
                attempts: r.get(3)?,
                message: StoredMessage {
                    client_msg_id: r.get(4)?,
                    seq: r.get(5)?,
                    role: r.get(6)?,
                    kind: r.get(7)?,
                    status: r.get(8)?,
                    content: r.get(9)?,
                    card_json: r.get(10)?,
                    provider: r.get(11)?,
                    attachment_count: r.get(12)?,
                    created_at: r.get(13)?,
                },
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn outbox_mark_synced(&self, outbox_id: i64) -> Result<(), StoreError> {
        self.connection.execute(
            &format!("UPDATE chat_outbox SET synced_at = {NOW}, last_error = NULL WHERE id = ?1"),
            params![outbox_id],
        )?;
        Ok(())
    }

    /// Record a failed attempt. `error` must be a short non-sensitive code, never response bodies.
    pub fn outbox_mark_failed(&self, outbox_id: i64, error: &str) -> Result<(), StoreError> {
        let error: String = error.chars().take(120).collect();
        self.connection.execute(
            "UPDATE chat_outbox SET attempts = attempts + 1, last_error = ?2 WHERE id = ?1",
            params![outbox_id, error],
        )?;
        Ok(())
    }

    pub fn outbox_unsent_count(&self) -> Result<i64, StoreError> {
        Ok(self.connection.query_row(
            "SELECT count(*) FROM chat_outbox WHERE synced_at IS NULL",
            [],
            |r| r.get(0),
        )?)
    }
}

fn read_message(r: &rusqlite::Row<'_>) -> rusqlite::Result<StoredMessage> {
    Ok(StoredMessage {
        client_msg_id: r.get(0)?,
        seq: r.get(1)?,
        role: r.get(2)?,
        kind: r.get(3)?,
        status: r.get(4)?,
        content: r.get(5)?,
        card_json: r.get(6)?,
        provider: r.get(7)?,
        attachment_count: r.get(8)?,
        created_at: r.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn store(name: &str) -> (SqlCipherStore, PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "retinacare-chat-{name}-{}-{}/db.sqlite",
            std::process::id(),
            rand::random::<u64>()
        ));
        (SqlCipherStore::open(&path, "pw").unwrap(), path)
    }

    fn text(role: &str, content: &str) -> MessageInput {
        MessageInput {
            client_msg_id: new_uuid(),
            role: role.into(),
            kind: "text".into(),
            status: "completed".into(),
            content: content.into(),
            card_json: None,
            provider: None,
            attachment_count: 0,
        }
    }

    #[test]
    fn uuid_format_is_v4() {
        for _ in 0..50 {
            let id = new_uuid();
            assert!(is_uuid(&id), "{id}");
            assert_eq!(&id[14..15], "4");
        }
        assert!(!is_uuid("not-a-uuid"));
        assert!(!is_uuid(&new_uuid().to_uppercase()));
    }

    #[test]
    fn messages_persist_across_reopen_and_duplicates_are_ignored() {
        let (s, path) = store("reopen");
        let conv = s
            .create_conversation("Hello", ConversationMode::Chat, None, None)
            .unwrap();
        let first = text("user", "hi there");
        assert!(s.append_message(&conv, &first).unwrap());
        assert!(!s.append_message(&conv, &first).unwrap(), "duplicate");
        assert!(
            s.append_message(&conv, &text("assistant", "reply"))
                .unwrap()
        );
        drop(s);
        let s = SqlCipherStore::open(&path, "pw").unwrap();
        let msgs = s.load_messages(&conv).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].seq, 1);
        assert_eq!(msgs[1].content, "reply");
        let list = s.list_conversations("").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].message_count, 2);
        assert_eq!(list[0].preview, "hi there");
        assert!(SqlCipherStore::open(&path, "wrong").is_err());
    }

    #[test]
    fn search_matches_title_and_text_and_escapes_wildcards() {
        let (s, _) = store("search");
        let a = s
            .create_conversation("Alpha", ConversationMode::Chat, None, None)
            .unwrap();
        s.append_message(&a, &text("user", "vessel question"))
            .unwrap();
        s.create_conversation("Beta 100%", ConversationMode::Chat, None, None)
            .unwrap();
        assert_eq!(s.list_conversations("VESSEL").unwrap().len(), 1);
        assert_eq!(s.list_conversations("beta").unwrap().len(), 1);
        assert_eq!(s.list_conversations("%").unwrap().len(), 1, "literal %");
        assert_eq!(s.list_conversations("_").unwrap().len(), 0);
        assert_eq!(s.list_conversations("").unwrap().len(), 2);
    }

    #[test]
    fn research_conversations_and_attachments_never_reach_the_outbox() {
        let (s, _) = store("outbox");
        let pid = s.add_patient("P-9", "Synthetic").unwrap();
        let eid = s.add_examination(pid, "op", "").unwrap();
        // A chat conversation cannot carry a patient.
        assert!(
            s.create_conversation("x", ConversationMode::Chat, Some(pid), None)
                .is_err()
        );
        // Examination must belong to the patient.
        let other = s.add_patient("P-10", "Synthetic").unwrap();
        assert!(
            s.create_conversation("x", ConversationMode::Research, Some(other), Some(eid))
                .is_err()
        );
        let research = s
            .create_conversation("r", ConversationMode::Research, Some(pid), Some(eid))
            .unwrap();
        assert!(s.set_conversation_sync(&research, true).is_err());

        let chat = s
            .create_conversation("c", ConversationMode::Chat, None, None)
            .unwrap();
        s.append_message(&chat, &text("user", "before sync"))
            .unwrap();
        s.set_conversation_sync(&chat, true).unwrap();
        s.append_message(&chat, &text("assistant", "after sync"))
            .unwrap();
        assert_eq!(s.outbox_pending(50).unwrap().len(), 2);

        let mut with_file = text("user", "see image");
        with_file.attachment_count = 1;
        s.append_message(&chat, &with_file).unwrap();
        assert_eq!(
            s.outbox_pending(50).unwrap().len(),
            0,
            "attachment clears the queue"
        );
        assert!(
            s.set_conversation_sync(&chat, true).is_err(),
            "stays local-only"
        );
    }

    #[test]
    fn delete_all_removes_every_conversation_and_message() {
        let (s, _) = store("delete-all");
        let a = s
            .create_conversation("a", ConversationMode::Chat, None, None)
            .unwrap();
        let b = s
            .create_conversation("b", ConversationMode::Chat, None, None)
            .unwrap();
        s.append_message(&a, &text("user", "hi")).unwrap();
        s.append_message(&b, &text("user", "yo")).unwrap();
        assert_eq!(s.delete_all_conversations().unwrap(), 2);
        assert!(s.list_conversations("").unwrap().is_empty());
        assert!(s.load_messages(&a).is_err() && s.load_messages(&b).is_err());
        assert_eq!(s.delete_all_conversations().unwrap(), 0);
    }

    #[test]
    fn outbox_marks_and_streaming_updates() {
        let (s, _) = store("marks");
        let chat = s
            .create_conversation("c", ConversationMode::Chat, None, None)
            .unwrap();
        s.set_conversation_sync(&chat, true).unwrap();
        let mut reply = text("assistant", "");
        reply.status = "streaming".into();
        s.append_message(&chat, &reply).unwrap();
        assert!(
            s.outbox_pending(10).unwrap().is_empty(),
            "unfinished replies wait"
        );
        assert!(
            s.update_message(&reply.client_msg_id, "done text", "completed", None)
                .unwrap()
        );
        let pending = s.outbox_pending(10).unwrap();
        assert_eq!(pending.len(), 1);
        s.outbox_mark_failed(pending[0].outbox_id, "network")
            .unwrap();
        assert_eq!(s.outbox_pending(10).unwrap()[0].attempts, 1);
        s.outbox_mark_synced(pending[0].outbox_id).unwrap();
        assert_eq!(s.outbox_unsent_count().unwrap(), 0);
        assert!(s.delete_conversation(&chat).unwrap());
        assert!(s.list_conversations("").unwrap().is_empty());
        assert!(s.load_messages(&chat).is_err());
    }

    #[test]
    fn a_pull_never_duplicates_never_resurrects_and_never_queues_uploads() {
        let (s, _) = store("import");
        let remote = new_uuid();
        let msgs: Vec<MessageInput> = (0..3)
            .map(|i| MessageInput {
                client_msg_id: new_uuid(),
                role: if i % 2 == 0 { "user" } else { "assistant" }.into(),
                kind: "text".into(),
                status: "completed".into(),
                content: format!("m{i}"),
                card_json: None,
                provider: None,
                attachment_count: 0,
            })
            .collect();
        assert_eq!(
            s.import_remote_conversation(&remote, "Remote", &msgs)
                .unwrap(),
            3
        );
        assert_eq!(
            s.import_remote_conversation(&remote, "Remote", &msgs)
                .unwrap(),
            0
        );
        assert_eq!(s.load_messages(&remote).unwrap().len(), 3);
        assert_eq!(
            s.outbox_pending(50).unwrap().len(),
            0,
            "imported rows are already synced"
        );
        assert!(s.conversation_known(&remote).unwrap());
        assert!(s.delete_conversation(&remote).unwrap());
        assert_eq!(
            s.import_remote_conversation(&remote, "Remote", &msgs)
                .unwrap(),
            0
        );
        assert!(
            s.list_conversations("").unwrap().is_empty(),
            "deleted stays deleted"
        );
        let mut bad = msgs[0].clone();
        bad.attachment_count = 1;
        assert!(
            s.import_remote_conversation(&new_uuid(), "x", &[bad])
                .is_err()
        );
        assert!(s.import_remote_conversation("not-uuid", "x", &[]).is_err());
    }

    #[test]
    fn invalid_input_is_refused() {
        let (s, _) = store("invalid");
        assert!(
            s.create_conversation("  ", ConversationMode::Chat, None, None)
                .is_err()
        );
        let c = s
            .create_conversation("ok", ConversationMode::Chat, None, None)
            .unwrap();
        let mut bad = text("user", "x");
        bad.client_msg_id = "1; DROP TABLE x".into();
        assert!(s.append_message(&c, &bad).is_err());
        let mut bad = text("robot", "x");
        bad.role = "robot".into();
        assert!(s.append_message(&c, &bad).is_err());
        assert!(s.append_message("missing", &text("user", "x")).is_err());
        let huge = "a".repeat(MAX_CONTENT + 1);
        assert!(s.append_message(&c, &text("user", &huge)).is_err());
    }
}
