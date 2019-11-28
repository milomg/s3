// THIS FILE IS NOT REFERENCED IN MAIN, IT IS A WIP

use rusqlite::{NO_PARAMS};
use failure::Error;
use r2d2_sqlite;
use r2d2_sqlite::SqliteConnectionManager;
use actix::prelude::*;

pub type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;
pub type Connection = r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>;

pub fn get_conn() -> Pool {
    let manager = SqliteConnectionManager::file("db/main.db");
    let pool = Pool::new(manager).unwrap();
    pool
}

pub fn get_users(conn: Connection) -> Result<(String, i16), Error> {
    let stmt = "
    SELECT * FROM users;";

    let mut prep_stmt = conn.prepare(stmt)?;
    let annuals = prep_stmt
        .query_map(NO_PARAMS, |row| Ok((row.get(0).unwrap(),row.get(1).unwrap())))
        .and_then(|mut mapped_rows| {
            Ok(mapped_rows.next().unwrap())
        })?;

    Ok(annuals.unwrap())
}

struct DbServer{
    pool: Pool
}

impl Actor for DbServer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {}
}

impl Handler<Disconnect> for DbServer {

}