use anyhow::{Context, Result};
use domain::Composition;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::DEFAULT_THEME_ID;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetSummary {
    pub id: String,
    pub name: String,
    pub theme_id: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetDetail {
    pub id: String,
    pub name: String,
    pub theme_id: String,
    pub composition: Composition,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePresetRequest {
    pub name: String,
    #[serde(default)]
    pub theme_id: Option<String>,
    pub composition: Composition,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdatePresetRequest {
    pub name: Option<String>,
    pub theme_id: Option<String>,
    pub composition: Option<Composition>,
}

pub fn list_presets(conn: &Connection) -> Result<Vec<PresetSummary>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, theme_id, updated_at FROM presets ORDER BY name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(PresetSummary {
            id: row.get(0)?,
            name: row.get(1)?,
            theme_id: row.get(2)?,
            updated_at: row.get(3)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn get_preset(conn: &Connection, id: &str) -> Result<Option<PresetDetail>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, theme_id, composition_json, created_at, updated_at
         FROM presets WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        let composition_json: String = row.get(3)?;
        let composition: Composition =
            serde_json::from_str(&composition_json).context("parse preset composition")?;
        return Ok(Some(PresetDetail {
            id: row.get(0)?,
            name: row.get(1)?,
            theme_id: row.get(2)?,
            composition,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        }));
    }
    Ok(None)
}

pub fn create_preset(conn: &Connection, req: CreatePresetRequest) -> Result<PresetDetail> {
    let id = Uuid::new_v4().to_string();
    let now = now_rfc3339();
    let theme_id = req.theme_id.unwrap_or_else(|| DEFAULT_THEME_ID.to_string());
    let composition_json = serde_json::to_string(&req.composition)?;
    conn.execute(
        "INSERT INTO presets (id, name, theme_id, composition_json, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, req.name, theme_id, composition_json, now, now],
    )?;
    get_preset(conn, &id)?.ok_or_else(|| anyhow::anyhow!("preset just inserted"))
}

pub fn update_preset(conn: &Connection, id: &str, req: UpdatePresetRequest) -> Result<PresetDetail> {
    let Some(existing) = get_preset(conn, id)? else {
        anyhow::bail!("preset not found");
    };
    let now = now_rfc3339();
    let name = req.name.unwrap_or(existing.name);
    let theme_id = req.theme_id.unwrap_or(existing.theme_id);
    let composition = req.composition.unwrap_or(existing.composition);
    let composition_json = serde_json::to_string(&composition)?;
    conn.execute(
        "UPDATE presets SET name = ?2, theme_id = ?3, composition_json = ?4, updated_at = ?5
         WHERE id = ?1",
        params![id, name, theme_id, composition_json, now],
    )?;
    get_preset(conn, id)?.ok_or_else(|| anyhow::anyhow!("preset missing after update"))
}

pub fn delete_preset(conn: &Connection, id: &str) -> Result<bool> {
    let n = conn.execute("DELETE FROM presets WHERE id = ?1", params![id])?;
    Ok(n > 0)
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

impl super::Store {
    pub fn list_presets(&self) -> Result<Vec<PresetSummary>> {
        list_presets(self.connection())
    }

    pub fn get_preset(&self, id: &str) -> Result<Option<PresetDetail>> {
        get_preset(self.connection(), id)
    }

    pub fn create_preset(&self, req: CreatePresetRequest) -> Result<PresetDetail> {
        create_preset(self.connection(), req)
    }

    pub fn update_preset(&self, id: &str, req: UpdatePresetRequest) -> Result<PresetDetail> {
        update_preset(self.connection(), id, req)
    }

    pub fn delete_preset(&self, id: &str) -> Result<bool> {
        delete_preset(self.connection(), id)
    }
}
