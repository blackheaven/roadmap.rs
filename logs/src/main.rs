use chrono::{
    prelude::{DateTime, Local, NaiveDate},
    Datelike,
};
use clap::Parser;

#[derive(Parser)] // requires `derive` feature
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum Cli {
    Add(AddArgs),
    Summary,
}

#[derive(clap::Args)]
#[command(version, about, long_about = None)]
struct AddArgs {
    #[arg(long)]
    at: Option<DateTime<Local>>,
}

// CREATE TABLE entries(id INTEGER PRIMARY KEY AUTOINCREMENT, at TEXT);

fn main() {
    let connection = sqlite::open("logs.db").expect("Cannot open db");

    match Cli::parse() {
        Cli::Add(args) => {
            let at = args.at.unwrap_or_else(|| Local::now());
            let query = "INSERT INTO entries(at) VALUES(?)";
            let mut statement = connection.prepare(query).unwrap();
            statement.bind((1, at.to_rfc3339().as_str())).unwrap();
            statement.next().unwrap();
            println!("Added {:?}", at);
        }
        Cli::Summary => {
            println!("Summary");
            let query = "SELECT date(at) as day, COUNT(*) as nb FROM entries GROUP BY date(at) ORDER BY day ASC";
            for row in connection
                .prepare(query)
                .unwrap()
                .into_iter()
                .map(|row| row.unwrap())
            {
                let day =
                    NaiveDate::parse_from_str(row.read::<&str, _>("day"), "%Y-%m-%d").unwrap();
                println!("{} | {} | {}", day, day.weekday(), row.read::<i64, _>("nb"));
            }
        }
    }
}
