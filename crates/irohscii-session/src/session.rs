//! Session management for irohscii
//!
//! Provides named sessions that bundle documents with metadata,
//! enabling ticket-based workflow switching between multiple drawings.

use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

use irohscii_core::Document;

// We define a simplified PeerId here instead of depending on irohscii-sync
// to avoid circular dependencies
/// Peer ID bytes (same format as sync crate)
#[derive(Debug, Clone, Copy)]
pub struct PeerId(pub [u8; 32]);

/// Maximum number of recent sessions to track in the registry
const MAX_RECENT_SESSIONS: usize = 50;

/// Session identifier - a URL-safe slug derived from the session name
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    /// Create a new SessionId from a name (converts to slug)
    pub fn from_name(name: &str) -> Self {
        let slug = name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");

        // Ensure we have something valid
        let slug = if slug.is_empty() {
            format!(
                "session-{}",
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            )
        } else {
            slug
        };

        Self(slug)
    }

    /// Get the raw slug string
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Information about a known collaborator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collaborator {
    pub peer_id: String,
    pub display_name: Option<String>,
    pub last_seen: u64, // Unix timestamp
}

/// Ticket information for reconnecting to a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketInfo {
    pub value: String,
    pub generated_at: u64, // Unix timestamp
}

/// Session metadata stored alongside the document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: SessionId,
    pub name: String,
    pub description: Option<String>,
    pub created_at: u64,    // Unix timestamp
    pub last_accessed: u64, // Unix timestamp
    pub ticket: Option<TicketInfo>,
    pub collaborators: Vec<Collaborator>,
    pub tags: Vec<String>,
    pub pinned: bool,
}

impl SessionMeta {
    /// Create new session metadata
    pub fn new(name: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: SessionId::from_name(name),
            name: name.to_string(),
            description: None,
            created_at: now,
            last_accessed: now,
            ticket: None,
            collaborators: Vec::new(),
            tags: Vec::new(),
            pinned: false,
        }
    }

    /// Update last accessed timestamp
    pub fn touch(&mut self) {
        self.last_accessed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Set the sync ticket
    pub fn set_ticket(&mut self, ticket: &str) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.ticket = Some(TicketInfo {
            value: ticket.to_string(),
            generated_at: now,
        });
    }

    /// Add or update a collaborator
    #[allow(dead_code)]
    pub fn add_collaborator(&mut self, peer_id: PeerId, display_name: Option<String>) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let peer_id_str = format!("{:?}", peer_id);

        // Update existing or add new
        if let Some(collab) = self
            .collaborators
            .iter_mut()
            .find(|c| c.peer_id == peer_id_str)
        {
            collab.last_seen = now;
            if display_name.is_some() {
                collab.display_name = display_name;
            }
        } else {
            self.collaborators.push(Collaborator {
                peer_id: peer_id_str,
                display_name,
                last_seen: now,
            });
        }
    }
}

/// Registry of sessions for quick access
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SessionRegistry {
    /// Recently accessed sessions (ordered by last_accessed, most recent first)
    pub recent: Vec<SessionId>,
    /// User's last active session
    pub last_active: Option<SessionId>,
}

impl SessionRegistry {
    /// Add a session to recent list (moves to front if exists)
    pub fn mark_accessed(&mut self, id: &SessionId) {
        self.recent.retain(|s| s != id);
        self.recent.insert(0, id.clone());
        self.recent.truncate(MAX_RECENT_SESSIONS);
        self.last_active = Some(id.clone());
    }

    /// Remove a session from the registry
    pub fn remove(&mut self, id: &SessionId) {
        self.recent.retain(|s| s != id);
        if self.last_active.as_ref() == Some(id) {
            self.last_active = self.recent.first().cloned();
        }
    }
}

/// Manages session storage and lifecycle
pub struct SessionManager {
    /// Base directory for all sessions
    sessions_dir: PathBuf,
    /// Registry file path
    registry_path: PathBuf,
    /// Cached registry
    registry: SessionRegistry,
}

impl SessionManager {
    /// Create a new SessionManager, initializing directories if needed
    pub fn new() -> Result<Self> {
        let data_dir = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".local/share")
            });

        let sessions_dir = data_dir.join("irohscii").join("sessions");
        let registry_path = data_dir.join("irohscii").join("registry.json");

        // Ensure directories exist
        fs::create_dir_all(&sessions_dir).context("Failed to create sessions directory")?;

        // Load or create registry
        let registry = if registry_path.exists() {
            let content = fs::read_to_string(&registry_path).context("Failed to read registry")?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            SessionRegistry::default()
        };

        Ok(Self {
            sessions_dir,
            registry_path,
            registry,
        })
    }

    /// Get the sessions directory path
    #[allow(dead_code)]
    pub fn sessions_dir(&self) -> &PathBuf {
        &self.sessions_dir
    }

    /// Save the registry to disk (atomic write via temp file)
    pub fn save_registry(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.registry)?;
        let temp_path = self.registry_path.with_extension("tmp");
        fs::write(&temp_path, &content).context("Failed to write temp registry")?;
        fs::rename(&temp_path, &self.registry_path).context("Failed to rename registry")?;
        Ok(())
    }

    /// Get the path for a session's directory
    fn session_dir(&self, id: &SessionId) -> PathBuf {
        self.sessions_dir.join(&id.0)
    }

    /// Get the path for a session's metadata file
    fn meta_path(&self, id: &SessionId) -> PathBuf {
        self.session_dir(id).join("session.json")
    }

    /// Get the path for a session's document file
    fn doc_path(&self, id: &SessionId) -> PathBuf {
        self.session_dir(id).join("document.automerge")
    }

    /// List all sessions with their metadata
    pub fn list_sessions(&self) -> Result<Vec<SessionMeta>> {
        let mut sessions = Vec::new();

        if !self.sessions_dir.exists() {
            return Ok(sessions);
        }

        for entry in fs::read_dir(&self.sessions_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let id = SessionId(entry.file_name().to_string_lossy().to_string());
                if let Ok(meta) = self.load_meta(&id) {
                    sessions.push(meta);
                }
            }
        }

        // Sort by last_accessed (most recent first), pinned first
        sessions.sort_by(|a, b| match (a.pinned, b.pinned) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.last_accessed.cmp(&a.last_accessed),
        });

        Ok(sessions)
    }

    /// Load session metadata
    pub fn load_meta(&self, id: &SessionId) -> Result<SessionMeta> {
        let path = self.meta_path(id);
        let content = fs::read_to_string(&path).context("Failed to read session metadata")?;
        serde_json::from_str(&content).context("Failed to parse session metadata")
    }

    /// Save session metadata (atomic write via temp file)
    pub fn save_meta(&self, meta: &SessionMeta) -> Result<()> {
        let dir = self.session_dir(&meta.id);
        fs::create_dir_all(&dir)?;

        let path = self.meta_path(&meta.id);
        let temp_path = path.with_extension("tmp");
        let content = serde_json::to_string_pretty(meta)?;
        fs::write(&temp_path, &content).context("Failed to write temp metadata")?;
        fs::rename(&temp_path, &path).context("Failed to rename metadata")?;
        Ok(())
    }

    /// Create a new session
    pub fn create_session(&mut self, name: &str) -> Result<SessionMeta> {
        // Validate session name
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("Session name cannot be empty"));
        }
        if trimmed.len() < 2 {
            return Err(anyhow!("Session name must be at least 2 characters"));
        }

        let mut meta = SessionMeta::new(trimmed);

        // Ensure unique ID by appending timestamp if needed
        let mut attempts = 0;
        while self.session_dir(&meta.id).exists() && attempts < 100 {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            meta.id = SessionId(format!("{}-{}", meta.id.0, now % 10000));
            attempts += 1;
        }

        if self.session_dir(&meta.id).exists() {
            return Err(anyhow!("Could not create unique session ID"));
        }

        // Create session directory
        let dir = self.session_dir(&meta.id);
        fs::create_dir_all(&dir)?;

        // Create empty document
        let mut doc = Document::new();
        let doc_path = self.doc_path(&meta.id);
        doc.save_to(&doc_path)?;

        // Save metadata
        self.save_meta(&meta)?;

        // Update registry
        self.registry.mark_accessed(&meta.id);
        self.save_registry()?;

        Ok(meta)
    }

    /// Open a session's document
    pub fn open_session(&mut self, id: &SessionId) -> Result<(Document, SessionMeta)> {
        let doc_path = self.doc_path(id);

        if !doc_path.exists() {
            return Err(anyhow!("Session document not found: {}", id));
        }

        // Load document
        let mut doc = Document::load(&doc_path)?;
        doc.set_storage_path(doc_path);

        // Load and update metadata
        let mut meta = self.load_meta(id)?;
        meta.touch();
        self.save_meta(&meta)?;

        // Update registry
        self.registry.mark_accessed(id);
        self.save_registry()?;

        Ok((doc, meta))
    }

    /// Save a session's document and metadata
    pub fn save_session(
        &self,
        id: &SessionId,
        doc: &mut Document,
        meta: &SessionMeta,
    ) -> Result<()> {
        let doc_path = self.doc_path(id);
        doc.save_to(&doc_path)?;
        self.save_meta(meta)?;
        Ok(())
    }

    /// Rename a session
    #[allow(dead_code)]
    pub fn rename_session(&mut self, id: &SessionId, new_name: &str) -> Result<SessionMeta> {
        let mut meta = self.load_meta(id)?;
        meta.name = new_name.to_string();
        // Note: We keep the same ID to preserve the directory
        self.save_meta(&meta)?;
        Ok(meta)
    }

    /// Delete a session
    pub fn delete_session(&mut self, id: &SessionId) -> Result<()> {
        let dir = self.session_dir(id);
        if dir.exists() {
            fs::remove_dir_all(&dir).context("Failed to delete session directory")?;
        }

        self.registry.remove(id);
        self.save_registry()?;

        Ok(())
    }

    /// Toggle pinned status
    pub fn toggle_pinned(&mut self, id: &SessionId) -> Result<bool> {
        let mut meta = self.load_meta(id)?;
        meta.pinned = !meta.pinned;
        self.save_meta(&meta)?;
        Ok(meta.pinned)
    }

    /// Get the last active session ID
    pub fn last_active(&self) -> Option<&SessionId> {
        self.registry.last_active.as_ref()
    }

    /// Check if a session exists
    pub fn session_exists(&self, id: &SessionId) -> bool {
        self.session_dir(id).exists()
    }

    /// Get session by name (fuzzy match)
    pub fn find_session(&self, query: &str) -> Result<Option<SessionMeta>> {
        let sessions = self.list_sessions()?;
        let query_lower = query.to_lowercase();

        // Exact match first
        if let Some(meta) = sessions
            .iter()
            .find(|m| m.id.0 == query || m.name.to_lowercase() == query_lower)
        {
            return Ok(Some(meta.clone()));
        }

        // Prefix match
        if let Some(meta) = sessions.iter().find(|m| {
            m.id.0.starts_with(&query_lower) || m.name.to_lowercase().starts_with(&query_lower)
        }) {
            return Ok(Some(meta.clone()));
        }

        // Contains match
        if let Some(meta) = sessions
            .iter()
            .find(|m| m.id.0.contains(&query_lower) || m.name.to_lowercase().contains(&query_lower))
        {
            return Ok(Some(meta.clone()));
        }

        Ok(None)
    }

    /// Filter sessions by a search query (fuzzy matching)
    #[allow(dead_code)]
    pub fn filter_sessions(&self, query: &str) -> Result<Vec<SessionMeta>> {
        let sessions = self.list_sessions()?;

        if query.is_empty() {
            return Ok(sessions);
        }

        let query_lower = query.to_lowercase();
        let query_chars: Vec<char> = query_lower.chars().collect();

        let mut scored: Vec<(SessionMeta, i32)> = sessions
            .into_iter()
            .filter_map(|meta| {
                let score = fuzzy_score(&meta.name.to_lowercase(), &query_chars)
                    .or_else(|| fuzzy_score(&meta.id.0, &query_chars))
                    .or_else(|| {
                        meta.tags
                            .iter()
                            .filter_map(|tag| fuzzy_score(&tag.to_lowercase(), &query_chars))
                            .max()
                    });

                score.map(|s| (meta, s))
            })
            .collect();

        // Sort by score (higher is better), then by pinned, then by last_accessed
        scored.sort_by(|a, b| match b.1.cmp(&a.1) {
            std::cmp::Ordering::Equal => match (a.0.pinned, b.0.pinned) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.0.last_accessed.cmp(&a.0.last_accessed),
            },
            other => other,
        });

        Ok(scored.into_iter().map(|(meta, _)| meta).collect())
    }
}

/// Simple fuzzy matching score (higher is better)
/// Returns None if no match
#[allow(dead_code)]
fn fuzzy_score(text: &str, query_chars: &[char]) -> Option<i32> {
    if query_chars.is_empty() {
        return Some(0);
    }

    let text_chars: Vec<char> = text.chars().collect();
    let mut query_idx = 0;
    let mut score = 0;
    let mut last_match_idx: Option<usize> = None;

    for (i, &c) in text_chars.iter().enumerate() {
        if query_idx < query_chars.len() && c == query_chars[query_idx] {
            // Bonus for consecutive matches
            if let Some(last) = last_match_idx && i == last + 1 {
                score += 10;
            }
            // Bonus for matching at word boundaries
            if i == 0 || !text_chars[i - 1].is_alphanumeric() {
                score += 5;
            }
            score += 1;
            last_match_idx = Some(i);
            query_idx += 1;
        }
    }

    if query_idx == query_chars.len() {
        Some(score)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_manager() -> (SessionManager, TempDir) {
        let temp = TempDir::new().unwrap();
        let sessions_dir = temp.path().join("sessions");
        let registry_path = temp.path().join("registry.json");

        fs::create_dir_all(&sessions_dir).unwrap();

        let manager = SessionManager {
            sessions_dir,
            registry_path,
            registry: SessionRegistry::default(),
        };

        (manager, temp)
    }

    #[test]
    fn test_session_id_from_name() {
        assert_eq!(SessionId::from_name("My Project").0, "my-project");
        assert_eq!(SessionId::from_name("PROJ-123").0, "proj-123");
        assert_eq!(SessionId::from_name("  hello   world  ").0, "hello-world");
        assert_eq!(SessionId::from_name("API/v2").0, "api-v2");
    }

    #[test]
    fn test_create_and_list_sessions() {
        let (mut manager, _temp) = test_manager();

        let meta1 = manager.create_session("First Session").unwrap();
        let meta2 = manager.create_session("Second Session").unwrap();

        let sessions = manager.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);

        // Most recent first
        assert_eq!(sessions[0].id, meta2.id);
        assert_eq!(sessions[1].id, meta1.id);
    }

    #[test]
    fn test_open_and_save_session() {
        let (mut manager, _temp) = test_manager();

        let created = manager.create_session("Test Session").unwrap();
        let (mut doc, meta) = manager.open_session(&created.id).unwrap();

        assert_eq!(meta.name, "Test Session");

        // Modify and save
        let _ = doc.add_shape(irohscii_core::ShapeKind::Rectangle {
            start: irohscii_core::Position::new(0, 0),
            end: irohscii_core::Position::new(10, 10),
            label: None,
            color: irohscii_core::ShapeColor::White,
        });

        manager.save_session(&created.id, &mut doc, &meta).unwrap();

        // Reopen and verify
        let (doc2, _) = manager.open_session(&created.id).unwrap();
        let shapes = doc2.read_all_shapes().unwrap();
        assert_eq!(shapes.len(), 1);
    }

    #[test]
    fn test_delete_session() {
        let (mut manager, _temp) = test_manager();

        let meta = manager.create_session("To Delete").unwrap();
        assert!(manager.session_exists(&meta.id));

        manager.delete_session(&meta.id).unwrap();
        assert!(!manager.session_exists(&meta.id));
    }

    #[test]
    fn test_fuzzy_score() {
        // Exact prefix should score high
        assert!(fuzzy_score("my-project", &['m', 'y']).unwrap() > 0);

        // Fuzzy match
        assert!(fuzzy_score("my-project", &['m', 'p']).is_some());

        // No match
        assert!(fuzzy_score("my-project", &['x', 'y', 'z']).is_none());

        // Consecutive bonus
        let consecutive = fuzzy_score("abc", &['a', 'b', 'c']).unwrap();
        let spread = fuzzy_score("a-b-c", &['a', 'b', 'c']).unwrap();
        assert!(consecutive > spread);
    }

    #[test]
    fn test_pinned_sessions() {
        let (mut manager, _temp) = test_manager();

        let meta1 = manager.create_session("Unpinned").unwrap();
        let meta2 = manager.create_session("Will Pin").unwrap();

        // Pin second session
        manager.toggle_pinned(&meta2.id).unwrap();

        let sessions = manager.list_sessions().unwrap();
        // Pinned should be first
        assert!(sessions[0].pinned);
        assert_eq!(sessions[0].id, meta2.id);
        assert!(!sessions[1].pinned);
        assert_eq!(sessions[1].id, meta1.id);
    }
}
