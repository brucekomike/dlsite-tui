use anyhow::Result;
use rusqlite::{Connection, params};

pub struct Work {
    pub id: String,
    pub name: String,
    pub circle: String,
    pub file_count: i64,
    pub purchase_date: String,
}

pub fn open_db() -> Result<Connection> {
    let conn = Connection::open("info.db")?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS works (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            circle TEXT NOT NULL DEFAULT '',
            file_count INTEGER NOT NULL DEFAULT 0,
            purchase_date TEXT NOT NULL DEFAULT ''
        );",
    )?;
    Ok(conn)
}

pub fn upsert_work(conn: &Connection, work: &Work) -> Result<()> {
    conn.execute(
        "INSERT INTO works (id, name, circle, file_count, purchase_date)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
            name=excluded.name,
            circle=excluded.circle,
            file_count=excluded.file_count,
            purchase_date=excluded.purchase_date",
        params![work.id, work.name, work.circle, work.file_count, work.purchase_date],
    )?;
    Ok(())
}

pub fn get_all_works(conn: &Connection) -> Result<Vec<Work>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, circle, file_count, purchase_date FROM works ORDER BY id ASC"
    )?;
    let works = stmt.query_map([], |row| {
        Ok(Work {
            id: row.get(0)?,
            name: row.get(1)?,
            circle: row.get(2)?,
            file_count: row.get(3)?,
            purchase_date: row.get(4)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;
    Ok(works)
}

pub fn get_all_works_by_date(conn: &Connection) -> Result<Vec<Work>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, circle, file_count, purchase_date FROM works ORDER BY purchase_date DESC"
    )?;
    let works = stmt.query_map([], |row| {
        Ok(Work {
            id: row.get(0)?,
            name: row.get(1)?,
            circle: row.get(2)?,
            file_count: row.get(3)?,
            purchase_date: row.get(4)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;
    Ok(works)
}

pub fn get_works_by_circle(conn: &Connection, circle: &str) -> Result<Vec<Work>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, circle, file_count, purchase_date FROM works WHERE circle=?1 ORDER BY id ASC"
    )?;
    let works = stmt.query_map(params![circle], |row| {
        Ok(Work {
            id: row.get(0)?,
            name: row.get(1)?,
            circle: row.get(2)?,
            file_count: row.get(3)?,
            purchase_date: row.get(4)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;
    Ok(works)
}

pub fn get_all_circles(conn: &Connection) -> Result<Vec<(String, usize)>> {
    let mut stmt = conn.prepare(
        "SELECT circle, COUNT(*) as cnt FROM works GROUP BY circle ORDER BY circle ASC"
    )?;
    let circles = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
    })?
    .collect::<Result<Vec<_>, _>>()?;
    Ok(circles)
}
