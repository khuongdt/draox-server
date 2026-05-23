use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use server_core::ClientId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceStatus {
    Online,
    Away,
    DoNotDisturb,
    Invisible,
    Offline,
    /// Game-specific status (e.g., "In-Game: Valorant")
    InGame { game: String },
    /// Fully custom status text set by the user
    Custom { text: String, emoji: Option<String> },
}

impl PresenceStatus {
    pub fn is_online(&self) -> bool {
        !matches!(self, PresenceStatus::Offline | PresenceStatus::Invisible)
    }

    pub fn display_label(&self) -> String {
        match self {
            PresenceStatus::Online => "Online".to_string(),
            PresenceStatus::Away => "Away".to_string(),
            PresenceStatus::DoNotDisturb => "Do Not Disturb".to_string(),
            PresenceStatus::Invisible => "Offline".to_string(),
            PresenceStatus::Offline => "Offline".to_string(),
            PresenceStatus::InGame { game } => format!("In-Game: {}", game),
            PresenceStatus::Custom { text, emoji } => match emoji {
                Some(e) => format!("{} {}", e, text),
                None => text.clone(),
            },
        }
    }
}

impl Default for PresenceStatus {
    fn default() -> Self {
        PresenceStatus::Offline
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPresence {
    pub client_id: ClientId,
    pub status: PresenceStatus,
    pub last_seen_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    pub status_message: Option<String>,
}

impl UserPresence {
    pub fn new(client_id: ClientId) -> Self {
        let now = Utc::now();
        Self {
            client_id,
            status: PresenceStatus::Offline,
            last_seen_at: now,
            last_activity_at: now,
            status_message: None,
        }
    }

    pub fn set_online(&mut self) {
        self.status = PresenceStatus::Online;
        let now = Utc::now();
        self.last_seen_at = now;
        self.last_activity_at = now;
    }

    pub fn set_offline(&mut self) {
        self.status = PresenceStatus::Offline;
        self.last_seen_at = Utc::now();
    }

    pub fn touch(&mut self) {
        self.last_activity_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_online_statuses() {
        assert!(PresenceStatus::Online.is_online());
        assert!(PresenceStatus::Away.is_online());
        assert!(PresenceStatus::DoNotDisturb.is_online());
        assert!(!PresenceStatus::Offline.is_online());
        assert!(!PresenceStatus::Invisible.is_online());
    }

    #[test]
    fn test_display_labels() {
        assert_eq!(PresenceStatus::Online.display_label(), "Online");
        assert_eq!(
            PresenceStatus::InGame { game: "Valorant".to_string() }.display_label(),
            "In-Game: Valorant"
        );
        assert_eq!(
            PresenceStatus::Custom {
                text: "Working".to_string(),
                emoji: Some("💼".to_string())
            }.display_label(),
            "💼 Working"
        );
    }
}
