use crate::db::Database;
use crate::models::user::{User, UserRole};
use rusqlite::params;
use std::str::FromStr;

impl Database {
    pub fn create_user(
        &self,
        username: &str,
        password_hash: &str,
        role: UserRole,
    ) -> Result<User, rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO users (username, password_hash, role) VALUES (?1, ?2, ?3)",
            params![username, password_hash, role.as_str()],
        )?;
        let id = conn.last_insert_rowid();
        drop(conn);
        self.get_user_by_id(id)
    }

    pub fn get_user_by_id(&self, id: i64) -> Result<User, rusqlite::Error> {
        self.conn().query_row(
            "SELECT id, username, password_hash, role, created_at FROM users WHERE id = ?1",
            params![id],
            |row| {
                Ok(User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    password_hash: row.get(2)?,
                    role: UserRole::from_str(&row.get::<_, String>(3)?).unwrap_or(UserRole::Viewer),
                    created_at: row.get(4)?,
                })
            },
        )
    }

    pub fn get_user_by_username(&self, username: &str) -> Result<User, rusqlite::Error> {
        self.conn().query_row(
            "SELECT id, username, password_hash, role, created_at FROM users WHERE username = ?1",
            params![username],
            |row| {
                Ok(User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    password_hash: row.get(2)?,
                    role: UserRole::from_str(&row.get::<_, String>(3)?).unwrap_or(UserRole::Viewer),
                    created_at: row.get(4)?,
                })
            },
        )
    }

    pub fn list_users(&self) -> Result<Vec<User>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, username, password_hash, role, created_at FROM users ORDER BY created_at DESC"
        )?;
        let users = stmt
            .query_map([], |row| {
                Ok(User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    password_hash: row.get(2)?,
                    role: UserRole::from_str(&row.get::<_, String>(3)?).unwrap_or(UserRole::Viewer),
                    created_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(users)
    }

    pub fn delete_user(&self, id: i64) -> Result<bool, rusqlite::Error> {
        let rows = self
            .conn()
            .execute("DELETE FROM users WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }
}
