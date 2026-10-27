//! Synchronous repository adapter. PostgreSQL's private runtime always runs outside Tokio.
use postgres::{
    row::RowIndex,
    types::{FromSqlOwned, ToSql},
    Client, NoTls,
};
use std::sync::Mutex;
#[macro_export]
macro_rules! params { ($($v:expr),* $(,)?) => { &[$(&$v as &(dyn postgres::types::ToSql + Sync)),*] }; }
pub struct Connection(Mutex<Option<Client>>);
impl Connection {
    pub fn open() -> Result<Self, Box<dyn std::error::Error>> {
        let url = std::env::var("PRM_DATABASE_URL")
            .map_err(|_| "Set PRM_DATABASE_URL to a local PostgreSQL database connection string")?;
        let config: postgres::Config = url.parse()?;
        // This application serves loopback only. Never send database credentials over a remote plaintext connection.
        if config.get_hosts().is_empty() || config.get_hosts().iter().any(|h| !matches!(h, postgres::config::Host::Tcp(s) if s == "localhost" || s == "127.0.0.1" || s == "::1")) {
            return Err("PostgreSQL must use an explicit loopback host".into());
        }
        let client = std::thread::spawn(move || config.connect(NoTls))
            .join()
            .expect("database connector panicked")?;
        Ok(Self(Mutex::new(Some(client))))
    }
    fn with<T: Send>(
        &self,
        f: impl FnOnce(&mut Client) -> Result<T, postgres::Error> + Send,
    ) -> Result<T, postgres::Error> {
        std::thread::scope(|s| {
            s.spawn(move || f(self.0.lock().unwrap().as_mut().unwrap()))
                .join()
                .expect("database worker panicked")
        })
    }
    pub fn execute_batch(&self, sql: &str) -> Result<(), postgres::Error> {
        self.with(|c| c.batch_execute(sql))
    }
    pub fn execute(&self, sql: &str, args: &[&(dyn ToSql + Sync)]) -> Result<u64, postgres::Error> {
        self.with(|c| c.execute(sql, args))
    }
    pub fn query_row<T>(
        &self,
        sql: &str,
        args: &[&(dyn ToSql + Sync)],
        f: impl FnOnce(&Row) -> Result<T, postgres::Error>,
    ) -> Result<T, postgres::Error> {
        let row = self.with(|c| c.query_one(sql, args))?;
        f(&Row(row))
    }
    pub fn prepare<'a>(&'a self, sql: &str) -> Result<Statement<'a>, postgres::Error> {
        Ok(Statement(self, sql.into()))
    }
}
impl Drop for Connection {
    fn drop(&mut self) {
        if let Some(client) = self.0.get_mut().unwrap().take() {
            std::thread::spawn(move || drop(client)).join().ok();
        }
    }
}
pub struct Row(postgres::Row);
impl Row {
    pub fn get<I: RowIndex + std::fmt::Display, T: FromSqlOwned>(
        &self,
        i: I,
    ) -> Result<T, postgres::Error> {
        self.0.try_get(i)
    }
}
pub struct Statement<'a>(&'a Connection, String);
impl Statement<'_> {
    pub fn query_map<T>(
        &mut self,
        args: &[&(dyn ToSql + Sync)],
        mut f: impl FnMut(&Row) -> Result<T, postgres::Error>,
    ) -> Result<std::vec::IntoIter<Result<T, postgres::Error>>, postgres::Error> {
        Ok(self
            .0
            .with(|c| c.query(&self.1, args))?
            .into_iter()
            .map(|r| f(&Row(r)))
            .collect::<Vec<_>>()
            .into_iter())
    }
}
