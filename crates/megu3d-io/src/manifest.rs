//! `manifest.json`: what the container is, who wrote it, and when.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Layout version of the `.megu3d` container itself. Bumped when entries move,
/// independently of the scene schema (`D-40`).
pub const CONTAINER_VERSION: u32 = 1;
/// Metres, Z-up (`D-30`).
pub const UNITS: &str = "m";
pub const APP_NAME: &str = "Megu3D";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    /// Scene schema of `scene.json`, not the app version.
    pub schema_version: String,
    pub container_version: u32,
    pub app: String,
    pub app_version: String,
    pub created: String,
    pub modified: String,
    pub units: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
}

impl Manifest {
    pub fn new(schema_version: impl Into<String>) -> Self {
        let now = now_rfc3339();
        Self {
            schema_version: schema_version.into(),
            container_version: CONTAINER_VERSION,
            app: APP_NAME.to_owned(),
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            created: now.clone(),
            modified: now,
            units: UNITS.to_owned(),
            thumbnail: None,
        }
    }

    /// Keeps `created` and the thumbnail from the file on disk and stamps a new
    /// `modified`. Saving must never rewrite the document's birthday.
    pub fn touched(&self, schema_version: impl Into<String>) -> Self {
        Self {
            schema_version: schema_version.into(),
            container_version: CONTAINER_VERSION,
            app: APP_NAME.to_owned(),
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            created: self.created.clone(),
            modified: now_rfc3339(),
            units: self.units.clone(),
            thumbnail: self.thumbnail.clone(),
        }
    }
}

pub fn now_rfc3339() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default();
    rfc3339(seconds)
}

/// UTC timestamp from a unix second count. No date crate: all the manifest
/// needs is a stable, sortable, human-readable string.
pub fn rfc3339(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let rest = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3_600,
        (rest % 3_600) / 60,
        rest % 60
    )
}

/// Howard Hinnant's `civil_from_days`: the standard branch-free conversion from
/// a day count to a civil date.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 { shifted } else { shifted - 146_096 } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_formats_as_rfc3339() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn a_known_timestamp_formats() {
        assert_eq!(rfc3339(1_700_000_000), "2023-11-14T22:13:20Z");
    }

    #[test]
    fn leap_days_are_handled() {
        assert_eq!(rfc3339(1_709_164_800), "2024-02-29T00:00:00Z");
    }

    #[test]
    fn the_manifest_is_camel_case_on_disk() {
        let manifest = Manifest::new("0.2.0");
        let json = serde_json::to_string(&manifest).expect("serialize");
        assert!(json.contains("\"schemaVersion\":\"0.2.0\""), "{json}");
        assert!(json.contains("\"containerVersion\":1"), "{json}");
        assert!(!json.contains("thumbnail"), "{json}");
        let parsed: Manifest = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed, manifest);
    }

    #[test]
    fn touching_keeps_the_creation_stamp() {
        let mut original = Manifest::new("0.1.0");
        original.created = "2020-01-01T00:00:00Z".to_owned();
        let touched = original.touched("0.2.0");
        assert_eq!(touched.created, "2020-01-01T00:00:00Z");
        assert_eq!(touched.schema_version, "0.2.0");
        assert!(touched.modified >= touched.created);
    }
}
