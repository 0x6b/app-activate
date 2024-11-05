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
ORDER BY count DESC
LIMIT 10
"#;

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
        let start_of_day = now.start_of_day()?;
        let start_of_7_days = start_of_day.saturating_sub(7.days());
        let start_of_30_days = start_of_day.saturating_sub(30.days());

        let list_for_day = self.select(&start_of_day, &now)?;
        let list_for_7_days = self.select(&start_of_7_days, &now)?;
        let list_for_30_days = self.select(&start_of_30_days, &now)?;

        // zip the three reports together line by line, and side by side
        let result = list_for_day
            .iter()
            .zip(list_for_7_days.iter())
            .zip(list_for_30_days.iter())
            .map(|((l, l7), l30)| format!("{l}    {l7}    {l30}"))
            .collect::<Vec<_>>();

        let format = "%Y-%m-%d";
        let now = now.strftime(format);

        println!("          Today                     Last 7 days                 Last 30 days");
        println!(
            " {} → {}      {} → {}      {} → {}",
            &start_of_day.strftime(format),
            &now,
            &start_of_7_days.strftime(format),
            &now,
            &start_of_30_days.strftime(format),
            &now,
        );
        println!("{}", result.join("\n"));

        Ok(())
    }

    fn select(&self, since: &Zoned, until: &Zoned) -> Result<Vec<String>> {
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

        let mut result: Vec<String> = Vec::new();
        result.push(format!("| {:col1w$} | {:col2w$} |", header.col1, header.col2));
        result.push(format!(
            "| {s1:col1w$} | {s2:col2w$}: |",
            s1 = "-".repeat(col1w),
            s2 = "-".repeat(col2w),
        ));
        iter.for_each(|r| result.push(format!("| {:col1w$} | {:>5} |", r.col1, r.col2)));

        Ok(result)
    }
}
