//! Schema migrations for the `scene.json` envelope.
//!
//! Migrations run on `serde_json::Value` before the document is typed, so an
//! old file never has to satisfy today's structs. Steps are a chain: every
//! version knows only the next one (`docs/02-architecture.md`, ADR-2).

use serde_json::{Map, Value};

use crate::IoError;

/// Schema this build writes. Always the core scene version.
pub const CURRENT: &str = megu3d_core::SCHEMA_VERSION;

/// One hop in the chain.
pub struct Step {
    pub from: &'static str,
    pub to: &'static str,
    pub apply: fn(&mut Map<String, Value>) -> Result<(), IoError>,
}

pub const STEPS: &[Step] = &[Step {
    from: "0.1.0",
    to: "0.2.0",
    apply: from_0_1_0,
}];

/// Upgrades a document to [`CURRENT`]. Newer files are refused rather than
/// guessed at: silently dropping unknown fields would lose user data.
pub fn migrate(document: Value, from: &str) -> Result<Value, IoError> {
    let found = parse(from)?;
    let current = parse(CURRENT)?;
    if found > current {
        return Err(IoError::SchemaTooNew {
            file: from.to_owned(),
            app: CURRENT.to_owned(),
        });
    }
    let Value::Object(mut object) = document else {
        return Err(IoError::Json("scene document is not an object".to_owned()));
    };
    let mut version = from.to_owned();
    while version != CURRENT {
        let step = STEPS
            .iter()
            .find(|step| step.from == version)
            .ok_or_else(|| IoError::SchemaUnsupported {
                file: version.clone(),
            })?;
        (step.apply)(&mut object)?;
        version = step.to.to_owned();
        object.insert(
            "schemaVersion".to_owned(),
            Value::String(version.clone()),
        );
    }
    Ok(Value::Object(object))
}

pub fn parse(version: &str) -> Result<[u32; 3], IoError> {
    let mut parts = [0_u32; 3];
    let mut seen = 0_usize;
    for (index, part) in version.split('.').enumerate() {
        if index >= parts.len() {
            return Err(unsupported(version));
        }
        parts[index] = part.parse::<u32>().map_err(|_| unsupported(version))?;
        seen = index + 1;
    }
    if seen != parts.len() {
        return Err(unsupported(version));
    }
    Ok(parts)
}

fn unsupported(version: &str) -> IoError {
    IoError::SchemaUnsupported {
        file: version.to_owned(),
    }
}

/// `0.1.0` kept the scene under `document` and had no unit field. Nothing was
/// released with it; the step exists so the chain has a tested shape before
/// real migrations arrive.
fn from_0_1_0(object: &mut Map<String, Value>) -> Result<(), IoError> {
    if let Some(document) = object.remove("document") {
        object.insert("scene".to_owned(), document);
    }
    if !object.contains_key("scene") {
        return Err(IoError::Json("0.1.0 document has no scene".to_owned()));
    }
    object
        .entry("units")
        .or_insert_with(|| Value::String("m".to_owned()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use megu3d_core::Scene;

    fn legacy_document() -> Value {
        let scene = serde_json::to_value(Scene::startup()).expect("scene");
        let mut object = Map::new();
        object.insert(
            "schemaVersion".to_owned(),
            Value::String("0.1.0".to_owned()),
        );
        object.insert("document".to_owned(), scene);
        Value::Object(object)
    }

    #[test]
    fn a_current_document_is_untouched() {
        let document = Value::Object(Map::new());
        assert_eq!(migrate(document.clone(), CURRENT).expect("migrate"), document);
    }

    #[test]
    fn legacy_documents_are_upgraded() {
        let migrated = migrate(legacy_document(), "0.1.0").expect("migrate");
        assert_eq!(
            migrated.get("schemaVersion").and_then(Value::as_str),
            Some(CURRENT)
        );
        assert_eq!(migrated.get("units").and_then(Value::as_str), Some("m"));
        assert!(migrated.get("scene").is_some(), "the scene moved under `scene`");
        assert!(migrated.get("document").is_none());
    }

    #[test]
    fn newer_documents_are_refused() {
        let error = migrate(Value::Object(Map::new()), "9.0.0").expect_err("must refuse");
        assert_eq!(error.code(), "IO_SCHEMA_TOO_NEW");
        assert!(error.recoverable());
    }

    #[test]
    fn unknown_versions_are_refused() {
        let error = migrate(Value::Object(Map::new()), "0.0.9").expect_err("must refuse");
        assert_eq!(error.code(), "IO_SCHEMA_UNSUPPORTED");
    }

    #[test]
    fn versions_must_be_semver() {
        assert_eq!(parse("0.2.0").expect("parse"), [0, 2, 0]);
        assert_eq!(
            parse("0.2").expect_err("must fail").code(),
            "IO_SCHEMA_UNSUPPORTED"
        );
        assert_eq!(
            parse("0.2.0.1").expect_err("must fail").code(),
            "IO_SCHEMA_UNSUPPORTED"
        );
        assert_eq!(
            parse("0.x.0").expect_err("must fail").code(),
            "IO_SCHEMA_UNSUPPORTED"
        );
    }

    #[test]
    fn every_step_moves_forward() {
        for step in STEPS {
            let from = parse(step.from).expect("from");
            let to = parse(step.to).expect("to");
            assert!(from < to, "{} -> {}", step.from, step.to);
        }
    }
}
