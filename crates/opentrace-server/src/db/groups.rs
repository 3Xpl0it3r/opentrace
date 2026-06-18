use crate::db::Database;
use crate::models::group::{CreateGroup, Group};
use rusqlite::params;

impl Database {
    pub fn create_group(&self, group: &CreateGroup) -> Result<Group, rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO agent_groups (name, description) VALUES (?1, ?2)",
            params![group.name, group.description],
        )?;
        let id = conn.last_insert_rowid();
        drop(conn);
        self.get_group_by_id(id)
    }

    pub fn get_group_by_id(&self, id: i64) -> Result<Group, rusqlite::Error> {
        self.conn().query_row(
            "SELECT id, name, description, created_at FROM agent_groups WHERE id = ?1",
            params![id],
            |row| {
                Ok(Group {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )
    }

    pub fn list_groups(&self) -> Result<Vec<Group>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_at FROM agent_groups ORDER BY created_at DESC",
        )?;
        let groups = stmt
            .query_map([], |row| {
                Ok(Group {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(groups)
    }

    pub fn delete_group(&self, id: i64) -> Result<bool, rusqlite::Error> {
        let rows = self
            .conn()
            .execute("DELETE FROM agent_groups WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }
}
