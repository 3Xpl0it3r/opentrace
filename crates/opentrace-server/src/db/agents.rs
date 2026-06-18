use crate::db::Database;
use crate::models::agent::{Agent, CreateAgent, UpdateAgent};
use rusqlite::params;

/// Helper: parse JSON string to Vec<Tracer>
fn parse_tracers(json: &str) -> Option<Vec<crate::models::agent::Tracer>> {
    if json.is_empty() {
        return None;
    }
    serde_json::from_str(json).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_agent_columns_allows_listing_empty_legacy_agents_table() {
        let db_path = std::env::temp_dir().join(format!(
            "opentrace-legacy-agents-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "
                CREATE TABLE agent_groups (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL UNIQUE,
                    description TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );

                CREATE TABLE agents (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    host TEXT NOT NULL
                );
                ",
            )
            .unwrap();
        }

        let db = Database::new(&db_path).unwrap();
        db.ensure_agent_columns().unwrap();

        let agents = db.list_agents(None, None).unwrap();

        assert!(agents.is_empty());

        drop(db);
        let _ = std::fs::remove_file(db_path);
    }
}

impl Database {
    /// Ensure migrations for existing DBs whose agents table predates newer UI fields.
    pub fn ensure_agent_columns(&self) -> Result<(), rusqlite::Error> {
        for (column, definition) in [
            ("group_id", "INTEGER REFERENCES agent_groups(id)"),
            ("status", "TEXT DEFAULT 'offline'"),
            ("tags", "TEXT"),
            ("cpu", "REAL"),
            ("memory", "REAL"),
            ("rate", "REAL"),
            ("uptime", "INTEGER"),
            ("version", "TEXT"),
            ("tracers", "TEXT"),
            ("token", "TEXT"),
            ("os", "TEXT"),
            ("arch", "TEXT"),
            ("created_at", "DATETIME"),
        ] {
            if !self.has_agent_column(column)? {
                self.conn().execute(
                    &format!("ALTER TABLE agents ADD COLUMN {column} {definition}"),
                    [],
                )?;
                eprintln!("[db] Added {column} column to agents table");
            }
        }
        self.conn().execute(
            "UPDATE agents SET created_at = CURRENT_TIMESTAMP WHERE created_at IS NULL",
            [],
        )?;
        Ok(())
    }

    fn has_agent_column(&self, column: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare("PRAGMA table_info(agents)")?;
        let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;

        for name in columns {
            if name? == column {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn create_agent(&self, agent: &CreateAgent) -> Result<Agent, rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO agents (name, host, group_id, tags, token, created_at) VALUES (?1, ?2, ?3, ?4, ?5, CURRENT_TIMESTAMP)",
            params![
                agent.name,
                agent.host,
                agent.group_id,
                agent.tags,
                agent.token
            ],
        )?;
        let id = conn.last_insert_rowid();
        drop(conn);
        self.get_agent_by_id(id)
    }

    /// Update agent with version and tracers info (called after /version fetch)
    pub fn update_agent_version(
        &self,
        id: i64,
        version: &str,
        tracers_json: &str,
    ) -> Result<(), rusqlite::Error> {
        self.conn().execute(
            "UPDATE agents SET version = ?1, tracers = ?2 WHERE id = ?3",
            params![version, tracers_json, id],
        )?;
        Ok(())
    }

    /// Update agent version, tracers AND status (called after /version fetch succeeds)
    pub fn update_agent_version_and_status(
        &self,
        id: i64,
        version: &str,
        tracers_json: &str,
        status: &str,
    ) -> Result<(), rusqlite::Error> {
        self.conn().execute(
            "UPDATE agents SET version = ?1, tracers = ?2, status = ?3 WHERE id = ?4",
            params![version, tracers_json, status, id],
        )?;
        Ok(())
    }

    pub fn update_agent_system_info(
        &self,
        id: i64,
        version: &str,
        tracers_json: &str,
        status: &str,
        os: &str,
        arch: &str,
    ) -> Result<(), rusqlite::Error> {
        self.conn().execute(
            "UPDATE agents SET version = ?1, tracers = ?2, status = ?3, os = ?4, arch = ?5 WHERE id = ?6",
            params![version, tracers_json, status, os, arch, id],
        )?;
        Ok(())
    }

    /// Update agent status only
    pub fn list_all_agents(&self) -> Result<Vec<Agent>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT a.id, a.name, a.host, a.group_id, g.name as group_name, a.status, a.tags, a.cpu, a.memory, a.rate, a.uptime, a.version, a.tracers, a.token, a.os, a.arch, a.created_at FROM agents a LEFT JOIN agent_groups g ON a.group_id = g.id"
        )?;
        let agents = stmt
            .query_map([], |row| {
                let tracers_json: Option<String> = row.get(12)?;
                Ok(Agent {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    host: row.get(2)?,
                    group_id: row.get(3)?,
                    group_name: row.get(4)?,
                    status: row.get(5)?,
                    tags: row.get(6)?,
                    cpu: row.get(7)?,
                    memory: row.get(8)?,
                    rate: row.get(9)?,
                    uptime: row.get(10)?,
                    version: row.get(11)?,
                    tracers: tracers_json.and_then(|j| serde_json::from_str(&j).ok()),
                    token: row.get(13)?,
                    os: row.get(14)?,
                    arch: row.get(15)?,
                    created_at: row.get(16)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(agents)
    }

    pub fn update_agent_status(&self, id: i64, status: &str) -> Result<(), rusqlite::Error> {
        self.conn().execute(
            "UPDATE agents SET status = ?1 WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    }

    pub fn count_agents(&self) -> Result<i64, rusqlite::Error> {
        self.conn()
            .query_row("SELECT COUNT(*) FROM agents", [], |row| row.get(0))
    }

    pub fn count_agents_by_status(&self, status: &str) -> Result<i64, rusqlite::Error> {
        self.conn().query_row(
            "SELECT COUNT(*) FROM agents WHERE status = ?1",
            params![status],
            |row| row.get(0),
        )
    }

    pub fn get_agent_by_id(&self, id: i64) -> Result<Agent, rusqlite::Error> {
        self.conn().query_row(
            "SELECT a.id, a.name, a.host, a.group_id, g.name as group_name, a.status, a.tags, a.cpu, a.memory, a.rate, a.uptime, a.version, a.tracers, a.token, a.os, a.arch, a.created_at FROM agents a LEFT JOIN agent_groups g ON a.group_id = g.id WHERE a.id = ?1",
            params![id],
            |row| {
                let tracers_json: Option<String> = row.get(12)?;
                Ok(Agent {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    host: row.get(2)?,
                    group_id: row.get(3)?,
                    group_name: row.get(4)?,
                    status: row.get(5)?,
                    tags: row.get(6)?,
                    cpu: row.get(7)?,
                    memory: row.get(8)?,
                    rate: row.get(9)?,
                    uptime: row.get(10)?,
                    version: row.get(11)?,
                    tracers: parse_tracers(&tracers_json.unwrap_or_default()),
                    token: row.get(13)?,
                    os: row.get(14)?,
                    arch: row.get(15)?,
                    created_at: row.get(16)?,
                })
            },
        )
    }

    pub fn list_agents(
        &self,
        group_id: Option<i64>,
        tag: Option<&str>,
    ) -> Result<Vec<Agent>, rusqlite::Error> {
        let mut sql = String::from(
            "SELECT a.id, a.name, a.host, a.group_id, g.name as group_name, a.status, a.tags, a.cpu, a.memory, a.rate, a.uptime, a.version, a.tracers, a.token, a.os, a.arch, a.created_at FROM agents a LEFT JOIN agent_groups g ON a.group_id = g.id WHERE 1=1",
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![];

        if let Some(gid) = group_id {
            sql.push_str(" AND a.group_id = ?");
            params.push(Box::new(gid));
        }
        if let Some(t) = tag {
            sql.push_str(" AND a.tags LIKE ?");
            params.push(Box::new(format!("%{}%", t)));
        }

        sql.push_str(" ORDER BY a.created_at DESC");

        let conn = self.conn();
        let mut stmt = conn.prepare(&sql)?;
        let agents = stmt
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                let tracers_json: Option<String> = row.get(12)?;
                Ok(Agent {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    host: row.get(2)?,
                    group_id: row.get(3)?,
                    group_name: row.get(4)?,
                    status: row.get(5)?,
                    tags: row.get(6)?,
                    cpu: row.get(7)?,
                    memory: row.get(8)?,
                    rate: row.get(9)?,
                    uptime: row.get(10)?,
                    version: row.get(11)?,
                    tracers: parse_tracers(&tracers_json.unwrap_or_default()),
                    token: row.get(13)?,
                    os: row.get(14)?,
                    arch: row.get(15)?,
                    created_at: row.get(16)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(agents)
    }

    pub fn update_agent(&self, id: i64, update: &UpdateAgent) -> Result<Agent, rusqlite::Error> {
        let current = self.get_agent_by_id(id)?;
        let name = update.name.as_deref().unwrap_or(&current.name);
        let host = update.host.as_deref().unwrap_or(&current.host);
        let group_id = update.group_id.or(current.group_id);
        let tags = update.tags.as_deref().or(current.tags.as_deref());
        let token = update.token.as_deref().or(current.token.as_deref());

        self.conn().execute(
            "UPDATE agents SET name = ?1, host = ?2, group_id = ?3, tags = ?4, token = ?5 WHERE id = ?6",
            params![name, host, group_id, tags, token, id],
        )?;
        self.get_agent_by_id(id)
    }

    pub fn delete_agent(&self, id: i64) -> Result<bool, rusqlite::Error> {
        let rows = self
            .conn()
            .execute("DELETE FROM agents WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }
}
