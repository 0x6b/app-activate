use std::{path::PathBuf, process::exit};

use anyhow::Result;
use jiff::{ToSpan, Zoned};
use log::error;
use prettytable::{
    format::{consts::FORMAT_NO_BORDER_LINE_SEPARATOR, Alignment},
    row, Cell, Table,
};
use rusqlite::Connection;

use crate::Config;

pub struct UsageReporter {
    conn: Connection,
}

const REPORT_QUERY: &str = r#"SELECT application, COUNT(application) AS count
FROM log
WHERE :since < datetime AND datetime < :until
GROUP BY application
ORDER BY count DESC"#;

impl UsageReporter {
    pub fn new(config: &Config) -> Result<Self> {
        let db = match &config.db {
            Some(db) => db,
            None => {
                error!("No database configuration");
                exit(1)
            }
        };

        Ok(Self { conn: Connection::open(db)? })
    }

    pub fn report(&self) -> Result<()> {
        let now = Zoned::now();

        self.display("Today", &now.start_of_day()?, &now)?;
        self.display("Last 7 days", &now.start_of_day()?.saturating_sub(7.days()), &now)?;
        self.display("Last 30 days", &now.start_of_day()?.saturating_sub(30.days()), &now)?;

        Ok(())
    }

    fn display(&self, title: &str, since: &Zoned, until: &Zoned) -> Result<()> {
        let mut table = Table::new();
        table.set_format(*FORMAT_NO_BORDER_LINE_SEPARATOR);
        table.set_titles(row!["Application", "Count"]);

        self.conn
            .prepare(REPORT_QUERY)?
            .query_map(
                [since.timestamp().as_second(), until.timestamp().as_second()],
                &Self::create_table_raw,
            )?
            .filter_map(|row| row.ok())
            .for_each(|row| {
                table.add_row(row);
            });

        println!("# {title}\n");
        table.printstd();
        println!();

        Ok(())
    }

    fn create_table_raw(row: &rusqlite::Row) -> Result<prettytable::Row, rusqlite::Error> {
        let path: String = row.get(0)?;
        let count: i64 = row.get(1)?;
        let application = PathBuf::from(path).file_stem().unwrap().to_string_lossy().to_string();

        Ok(prettytable::Row::new(vec![
            Cell::new(&application),
            Cell::new_align(&count.to_string(), Alignment::RIGHT),
        ]))
    }
}
