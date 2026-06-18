use crate::db::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tracepoint {
    pub id: i64,
    pub agent_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub sink_id: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTracepoint {
    pub name: String,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub sink_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTracepoint {
    pub enabled: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_sink_id")]
    pub sink_id: Option<Option<i64>>,
}

fn deserialize_optional_sink_id<'de, D>(deserializer: D) -> Result<Option<Option<i64>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<i64>::deserialize(deserializer).map(Some)
}

impl Database {
    pub fn ensure_tracepoint_sink_id_column(&self) -> Result<(), rusqlite::Error> {
        if !self.has_tracepoint_column("sink_id")? {
            self.conn().execute(
                "ALTER TABLE tracepoints ADD COLUMN sink_id INTEGER REFERENCES sinks(id) ON DELETE SET NULL",
                [],
            )?;
            eprintln!("[db] Added sink_id column to tracepoints table");
        }
        Ok(())
    }

    fn has_tracepoint_column(&self, column: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare("PRAGMA table_info(tracepoints)")?;
        let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;

        for name in columns {
            if name? == column {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn create_tracepoint(
        &self,
        agent_id: i64,
        tp: &CreateTracepoint,
    ) -> Result<Tracepoint, rusqlite::Error> {
        let enabled = tp.enabled.unwrap_or(true);
        let conn = self.conn();
        conn.execute(
            "INSERT INTO tracepoints (agent_id, name, description, enabled, sink_id) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![agent_id, tp.name, tp.description, enabled as i32, tp.sink_id],
        )?;
        let id = conn.last_insert_rowid();
        drop(conn);
        self.get_tracepoint_by_id(id)
    }

    pub fn get_tracepoint_by_id(&self, id: i64) -> Result<Tracepoint, rusqlite::Error> {
        self.conn().query_row(
            "SELECT id, agent_id, name, description, enabled, sink_id, created_at FROM tracepoints WHERE id = ?1",
            params![id],
            |row| {
                Ok(Tracepoint {
                    id: row.get(0)?,
                    agent_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    enabled: row.get::<_, i32>(4)? != 0,
                    sink_id: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        )
    }

    /// Internal: list all tracepoints for an agent without pagination
    pub fn list_all_tracepoints_for_agent(
        &self,
        agent_id: i64,
    ) -> Result<Vec<Tracepoint>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, name, description, enabled, sink_id, created_at FROM tracepoints WHERE agent_id = ?1 ORDER BY id"
        )?;
        let tps = stmt
            .query_map(params![agent_id], |row| {
                Ok(Tracepoint {
                    id: row.get(0)?,
                    agent_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    enabled: row.get::<_, i32>(4)? != 0,
                    sink_id: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tps)
    }

    pub fn list_tracepoints(
        &self,
        agent_id: i64,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<Tracepoint>, rusqlite::Error> {
        let conn = self.conn();
        let offset = (page.max(1) - 1) * page_size.max(1);
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, name, description, enabled, sink_id, created_at FROM tracepoints WHERE agent_id = ?1 ORDER BY id LIMIT ?2 OFFSET ?3"
        )?;
        let tps = stmt
            .query_map(params![agent_id, page_size, offset], |row| {
                Ok(Tracepoint {
                    id: row.get(0)?,
                    agent_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    enabled: row.get::<_, i32>(4)? != 0,
                    sink_id: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tps)
    }

    pub fn count_tracepoints_for_agent(&self, agent_id: i64) -> Result<i64, rusqlite::Error> {
        self.conn().query_row(
            "SELECT COUNT(*) FROM tracepoints WHERE agent_id = ?1",
            params![agent_id],
            |row| row.get(0),
        )
    }

    pub fn update_tracepoint(
        &self,
        agent_id: i64,
        id: i64,
        update: &UpdateTracepoint,
    ) -> Result<Tracepoint, rusqlite::Error> {
        let current = self.get_tracepoint_by_agent(agent_id, id)?;
        let enabled = update.enabled.unwrap_or(current.enabled);
        let sink_id = update.sink_id.unwrap_or(current.sink_id);
        self.conn().execute(
            "UPDATE tracepoints SET enabled = ?1, sink_id = ?2 WHERE id = ?3 AND agent_id = ?4",
            params![enabled as i32, sink_id, id, agent_id],
        )?;
        self.get_tracepoint_by_agent(agent_id, id)
    }

    pub fn delete_tracepoint(&self, agent_id: i64, id: i64) -> Result<bool, rusqlite::Error> {
        let rows = self.conn().execute(
            "DELETE FROM tracepoints WHERE id = ?1 AND agent_id = ?2",
            params![id, agent_id],
        )?;
        Ok(rows > 0)
    }

    fn get_tracepoint_by_agent(
        &self,
        agent_id: i64,
        id: i64,
    ) -> Result<Tracepoint, rusqlite::Error> {
        self.conn().query_row(
            "SELECT id, agent_id, name, description, enabled, sink_id, created_at FROM tracepoints WHERE id = ?1 AND agent_id = ?2",
            params![id, agent_id],
            |row| {
                Ok(Tracepoint {
                    id: row.get(0)?,
                    agent_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    enabled: row.get::<_, i32>(4)? != 0,
                    sink_id: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        )
    }

    pub fn get_tracepoint_by_agent_and_name(
        &self,
        agent_id: i64,
        name: &str,
    ) -> Result<Tracepoint, rusqlite::Error> {
        self.conn().query_row(
            "SELECT id, agent_id, name, description, enabled, sink_id, created_at FROM tracepoints WHERE agent_id = ?1 AND name = ?2",
            params![agent_id, name],
            |row| {
                Ok(Tracepoint {
                    id: row.get(0)?,
                    agent_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    enabled: row.get::<_, i32>(4)? != 0,
                    sink_id: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        )
    }

    /// Enable or disable a tracepoint by name (for a specific agent)
    pub fn enable_tracepoint_by_name(
        &self,
        agent_id: i64,
        name: &str,
        enabled: bool,
    ) -> Result<(), rusqlite::Error> {
        self.conn().execute(
            "UPDATE tracepoints SET enabled = ?1 WHERE agent_id = ?2 AND name = ?3",
            params![enabled as i32, agent_id, name],
        )?;
        Ok(())
    }

    pub fn set_tracepoint_enabled(&self, agent_id: i64, name: &str, enabled: bool) -> Result<(), rusqlite::Error> {
        self.conn().execute(
            "UPDATE tracepoints SET enabled = ?1 WHERE agent_id = ?2 AND name = ?3",
            params![enabled, agent_id, name],
        )?;
        Ok(())
    }

    pub fn set_all_tracepoints_enabled(&self, agent_id: i64, enabled: bool) -> Result<(), rusqlite::Error> {
        self.conn().execute(
            "UPDATE tracepoints SET enabled = ?1 WHERE agent_id = ?2",
            params![enabled, agent_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::new(std::path::Path::new(":memory:")).unwrap()
    }

    fn insert_agent(db: &Database, id: i64, name: &str) {
        db.conn()
            .execute(
                "INSERT INTO agents (id, name, host) VALUES (?1, ?2, ?3)",
                params![id, name, "127.0.0.1"],
            )
            .unwrap();
    }

    fn insert_sink(db: &Database, id: i64, name: &str) {
        db.conn()
            .execute(
                "INSERT INTO sinks (id, name, type, config) VALUES (?1, ?2, ?3, ?4)",
                params![id, name, "kafka", "{}"],
            )
            .unwrap();
    }

    #[test]
    fn update_tracepoint_requires_matching_agent() {
        let db = test_db();
        insert_agent(&db, 1, "agent-1");
        insert_agent(&db, 2, "agent-2");
        let tracepoint = db
            .create_tracepoint(
                1,
                &CreateTracepoint {
                    name: "skbdrop".to_string(),
                    description: None,
                    enabled: Some(true),
                    sink_id: None,
                },
            )
            .unwrap();

        let err = db
            .update_tracepoint(
                2,
                tracepoint.id,
                &UpdateTracepoint {
                    enabled: Some(false),
                    sink_id: None,
                },
            )
            .expect_err("cross-agent tracepoint update should fail");

        assert!(matches!(err, rusqlite::Error::QueryReturnedNoRows));
    }

    #[test]
    fn update_tracepoint_saves_sink_id() {
        let db = test_db();
        insert_agent(&db, 1, "agent-1");
        insert_sink(&db, 10, "kafka-main");
        let tracepoint = db
            .create_tracepoint(
                1,
                &CreateTracepoint {
                    name: "skbdrop".to_string(),
                    description: None,
                    enabled: Some(true),
                    sink_id: None,
                },
            )
            .unwrap();

        let updated = db
            .update_tracepoint(
                1,
                tracepoint.id,
                &UpdateTracepoint {
                    enabled: None,
                    sink_id: Some(Some(10)),
                },
            )
            .unwrap();

        assert_eq!(updated.sink_id, Some(10));
        assert!(updated.enabled);
    }

    #[test]
    fn update_tracepoint_preserves_sink_id_when_omitted() {
        let db = test_db();
        insert_agent(&db, 1, "agent-1");
        insert_sink(&db, 10, "kafka-main");
        let tracepoint = db
            .create_tracepoint(
                1,
                &CreateTracepoint {
                    name: "skbdrop".to_string(),
                    description: None,
                    enabled: Some(true),
                    sink_id: Some(10),
                },
            )
            .unwrap();

        let updated = db
            .update_tracepoint(
                1,
                tracepoint.id,
                &UpdateTracepoint {
                    enabled: Some(false),
                    sink_id: None,
                },
            )
            .unwrap();

        assert_eq!(updated.sink_id, Some(10));
        assert!(!updated.enabled);
    }

    #[test]
    fn update_tracepoint_clears_sink_id_when_null() {
        let db = test_db();
        insert_agent(&db, 1, "agent-1");
        insert_sink(&db, 10, "kafka-main");
        let tracepoint = db
            .create_tracepoint(
                1,
                &CreateTracepoint {
                    name: "skbdrop".to_string(),
                    description: None,
                    enabled: Some(true),
                    sink_id: Some(10),
                },
            )
            .unwrap();

        let updated = db
            .update_tracepoint(
                1,
                tracepoint.id,
                &UpdateTracepoint {
                    enabled: None,
                    sink_id: Some(None),
                },
            )
            .unwrap();

        assert_eq!(updated.sink_id, None);
        assert!(updated.enabled);
    }

    #[test]
    fn delete_tracepoint_requires_matching_agent() {
        let db = test_db();
        insert_agent(&db, 1, "agent-1");
        insert_agent(&db, 2, "agent-2");
        let tracepoint = db
            .create_tracepoint(
                1,
                &CreateTracepoint {
                    name: "skbdrop".to_string(),
                    description: None,
                    enabled: Some(true),
                    sink_id: None,
                },
            )
            .unwrap();

        let deleted = db.delete_tracepoint(2, tracepoint.id).unwrap();

        assert!(!deleted);
        assert!(db.get_tracepoint_by_id(tracepoint.id).is_ok());
    }

}