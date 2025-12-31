use std::{path::PathBuf, process::exit};

use anyhow::Result;
use jiff::{ToSpan, Zoned};
use log::error;
use rusqlite::{Connection, Error};

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

struct Row {
    col1: String,
    col2: String,
}

impl UsageReporter {
    pub fn new(config: &Config) -> Result<Self> {
        let Some(db) = &config.db else {
            error!("No database configuration");
            exit(1);
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

        let width = list_for_day
            .iter()
            .chain(list_for_7_days.iter())
            .chain(list_for_30_days.iter())
            .map(|s| s.len())
            .max()
            .unwrap_or(0);

        let format = "%Y-%m-%d";
        let now = now.strftime(format);

        println!(" {:width$}  {:width$}  {:width$}", "Today", "Last 7 days", "Last 30 days");
        println!(
            " {:width$}  {:width$}  {:width$}",
            format!("{} → {}", &start_of_day.strftime(format), &now),
            format!("{} → {}", &start_of_7_days.strftime(format), &now),
            format!("{} → {}", &start_of_30_days.strftime(format), &now)
        );

        for ((l, l7), l30) in list_for_day
            .iter()
            .zip(list_for_7_days.iter())
            .zip(list_for_30_days.iter())
        {
            println!("{l:width$}  {l7:width$}  {l30:width$}");
        }

        Ok(())
    }

    fn select(&self, since: &Zoned, until: &Zoned) -> Result<Vec<String>> {
        let mut table = vec![Row { col1: "Application".into(), col2: "Count".into() }];

        table.extend(
            self.conn
                .prepare(REPORT_QUERY)?
                .query_map(
                    [since.timestamp().as_second(), until.timestamp().as_second()],
                    |row| -> Result<Row, Error> {
                        let path: String = row.get(0)?;
                        let count: i64 = row.get(1)?;
                        Ok(Row {
                            col1: PathBuf::from(path)
                                .file_stem()
                                .unwrap()
                                .to_string_lossy()
                                .into_owned(),
                            col2: count.to_string(),
                        })
                    },
                )?
                .filter_map(Result::ok),
        );

        let col1w = table.iter().map(|r| r.col1.len()).max().unwrap();
        let col2w = table.iter().map(|r| r.col2.len()).max().unwrap() - 1;

        let mut iter = table.into_iter();
        let header = iter.next().unwrap();

        let mut result = Vec::with_capacity(12);
        result.push(format!("| {:col1w$} | {:col2w$} |", header.col1, header.col2));
        result.push(format!(
            "| {s1:col1w$} | {s2:col2w$}: |",
            s1 = "-".repeat(col1w),
            s2 = "-".repeat(col2w),
        ));
        result.extend(iter.map(|r| format!("| {:col1w$} | {:>5} |", r.col1, r.col2)));
        result.resize_with(12, || format!("| {:col1w$} | {:>5} |", "-", "-"));

        Ok(result)
    }
}
