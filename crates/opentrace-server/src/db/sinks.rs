use crate::db::Database;
use crate::models::sink::{CreateSink, Sink, UpdateSink};
use rusqlite::params;

impl Database {
    pub fn create_sink(&self, sink: &CreateSink) -> Result<Sink, rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO sinks (name, type, config) VALUES (?1, ?2, ?3)",
            params![sink.name, sink.sink_type, sink.config],
        )?;
        let id = conn.last_insert_rowid();
        drop(conn);
        self.get_sink_by_id(id)
    }

    pub fn get_sink_by_id(&self, id: i64) -> Result<Sink, rusqlite::Error> {
        self.conn().query_row(
            "SELECT id, name, type, config, status, created_at FROM sinks WHERE id = ?1",
            params![id],
            |row| {
                Ok(Sink {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    sink_type: row.get(2)?,
                    config: row.get(3)?,
                    status: row.get(4)?,
                    created_at: row.get(5)?,
                })
            },
        )
    }

    pub fn list_sinks(&self) -> Result<Vec<Sink>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, type, config, status, created_at FROM sinks ORDER BY created_at DESC",
        )?;
        let sinks = stmt
            .query_map([], |row| {
                Ok(Sink {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    sink_type: row.get(2)?,
                    config: row.get(3)?,
                    status: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(sinks)
    }

    pub fn update_sink(&self, id: i64, update: &UpdateSink) -> Result<Sink, rusqlite::Error> {
        let current = self.get_sink_by_id(id)?;
        let name = update.name.as_deref().unwrap_or(&current.name);
        let sink_type = update.sink_type.as_deref().unwrap_or(&current.sink_type);
        let config = update.config.as_deref().unwrap_or(&current.config);

        self.conn().execute(
            "UPDATE sinks SET name = ?1, type = ?2, config = ?3 WHERE id = ?4",
            params![name, sink_type, config, id],
        )?;
        self.get_sink_by_id(id)
    }

    pub fn delete_sink(&self, id: i64) -> Result<bool, rusqlite::Error> {
        let rows = self
            .conn()
            .execute("DELETE FROM sinks WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    pub fn get_sink_name_by_id(&self, id: i64) -> Result<Option<String>, rusqlite::Error> {
        let result = self.conn().query_row(
            "SELECT name FROM sinks WHERE id = ?1",
            params![id],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(name) => Ok(Some(name)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn bind_agent_to_sink(&self, sink_id: i64, agent_id: i64) -> Result<(), rusqlite::Error> {
        self.conn().execute(
            "INSERT OR IGNORE INTO sink_agent_bindings (sink_id, agent_id) VALUES (?1, ?2)",
            params![sink_id, agent_id],
        )?;
        Ok(())
    }

    pub fn unbind_agent_from_sink(
        &self,
        sink_id: i64,
        agent_id: i64,
    ) -> Result<bool, rusqlite::Error> {
        let rows = self.conn().execute(
            "DELETE FROM sink_agent_bindings WHERE sink_id = ?1 AND agent_id = ?2",
            params![sink_id, agent_id],
        )?;
        Ok(rows > 0)
    }

    pub fn get_sink_agents(&self, sink_id: i64) -> Result<Vec<i64>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt =
            conn.prepare("SELECT agent_id FROM sink_agent_bindings WHERE sink_id = ?1")?;
        let agents = stmt
            .query_map(params![sink_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(agents)
    }

    pub fn count_sinks(&self) -> Result<i64, rusqlite::Error> {
        self.conn()
            .query_row("SELECT COUNT(*) FROM sinks", [], |row| row.get(0))
    }

    pub fn count_sinks_by_status(&self, status: &str) -> Result<i64, rusqlite::Error> {
        self.conn().query_row(
            "SELECT COUNT(*) FROM sinks WHERE status = ?1",
            params![status],
            |row| row.get(0),
        )
    }

    pub fn update_sink_status(&self, id: i64, status: &str) -> Result<(), rusqlite::Error> {
        self.conn().execute(
            "UPDATE sinks SET status = ?1 WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    }
}
