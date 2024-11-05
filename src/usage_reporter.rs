use std::{path::PathBuf, process::exit};

use anyhow::Result;
use jiff::{ToSpan, Zoned};
use log::error;
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
        struct Row {
            col1: String,
            col2: String,
        }

        let mut table = vec![Row {
            col1: "Application".to_string(),
            col2: "Count".to_string(),
        }];
        table.extend(
            self.conn
                .prepare(REPORT_QUERY)?
                .query_map(
                    [since.timestamp().as_second(), until.timestamp().as_second()],
                    |row| -> Result<Row, rusqlite::Error> {
                        let path: String = row.get(0)?;
                        let count: i64 = row.get(1)?;

                        Ok(Row {
                            col1: PathBuf::from(path)
                                .file_stem()
                                .unwrap()
                                .to_string_lossy()
                                .to_string(),
                            col2: count.to_string(),
                        })
                    },
                )?
                .filter_map(|row| row.ok())
                .collect::<Vec<_>>(),
        );

        let col1w = table.iter().max_by_key(|data| data.col1.len()).unwrap().col1.len();
        let col2w = table.iter().max_by_key(|data| data.col2.len()).unwrap().col2.len() - 1;

        let mut iter = table.into_iter();
        let header = iter.next().unwrap();

        println!("# {title}\n");

        println!(
            "Since {} until {}",
            since.strftime("%Y-%m-%d %H:%M:%S%:z"),
            until.strftime("%Y-%m-%d %H:%M:%S%:z")
        );
        println!();
        println!("| {:col1w$} | {:col2w$} |", header.col1, header.col2);
        println!("| {s1:col1w$} | {s2:col2w$}: |", s1 = "-".repeat(col1w), s2 = "-".repeat(col2w),);
        iter.for_each(|r| println!("| {:col1w$} | {:>5} |", r.col1, r.col2));
        println!();

        Ok(())
    }
}
