use chrono::{DateTime, NaiveDate};
use colorous::*;
use plotters::{
    backend::BitMapBackend,
    drawing::IntoDrawingArea,
    element::PathElement,
    prelude::{ChartBuilder, LabelAreaPosition, LineSeries, RGBColor, BLUE, WHITE},
};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

struct Entry {
    date: NaiveDate,
    value: f64,
}

struct Summary(String, RGBColor, Vec<Entry>);

fn main() {
    let mut datapoints_by_command = HashMap::new();

    let mut position = 0;

    if let Ok(lines) = read_lines("/home/carlos/dockers/mongod.small.log") {
        for line in lines {
            if let Ok(ip) = line {
                let v = serde_json::from_str(ip.as_str());

                let linea: Value = match v {
                    Ok(l) => l,
                    Err(e) => panic!("{}", e),
                };

                let date_str = &linea["t"]["$date"];

                let date_value = match DateTime::parse_from_rfc3339(date_str.as_str().unwrap()) {
                    Ok(newdate) => newdate,
                    Err(err) => panic!("Unknown log format. Cannot get entry date {}", err),
                };

                let random_date = date_value
                    .checked_add_days(chrono::naive::Days::new(position))
                    .unwrap();

                position += 1;
                if position > 9 {
                    position = 0;
                }

                if let Some(attr) = linea.get("attr") {
                    if let Some(cmd) = attr.get("command") {
                        let command = find_command(cmd);
                        if command.is_none() {
                            continue;
                        }

                        let points: &mut HashMap<NaiveDate, f64> = datapoints_by_command
                            .entry(command.unwrap())
                            .or_insert(HashMap::new());

                        let count = points
                            .entry(random_date.date_naive())
                            .or_insert(f64::from(0));

                        *count += 1.0;
                    }
                }
            }
        }
    }

    let start_date = NaiveDate::from_ymd_opt(2023, 05, 25).unwrap();
    let end_date = NaiveDate::from_ymd_opt(2023, 06, 06).unwrap();

    let root_area = BitMapBackend::new("images/2.11.png", (2000, 2000)).into_drawing_area();
    root_area.fill(&WHITE).unwrap();
    let mut ctx = ChartBuilder::on(&root_area)
        .set_label_area_size(LabelAreaPosition::Left, 40)
        .set_label_area_size(LabelAreaPosition::Bottom, 800)
        .caption("MongoDB server executed commands", ("sans-serif", 40))
        .build_cartesian_2d(start_date..end_date, 0.0..120.0)
        .unwrap();

    ctx.configure_mesh().x_labels(10).draw().unwrap();

    let mut i = 0;
    let mut summary: Vec<Summary> = Vec::new();
    for (command, data_points) in &datapoints_by_command {
        let mut v: Vec<Entry> = Vec::new();
        for (date, value) in data_points {
            v.push(Entry {
                date: *date,
                value: *value,
            });
            v.sort_by(|a, b| a.date.cmp(&b.date));
        }
        let Color { r, g, b } = colorous::CATEGORY10[i];
        summary.push(Summary(command.to_string(), RGBColor(r, g, b), v));
        i += 1;
    }

    for serie in summary {
        ctx.draw_series(LineSeries::new(
            (0..).zip(serie.2.iter()).map(|(_, value)| {
                println!("{} -> {}", value.date, value.value);
                (value.date, value.value)
            }),
            &serie.1,
        ))
        .unwrap()
        .label(serie.0)
        .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], serie.1));
    }

    ctx.configure_series_labels()
        .border_style(&BLUE)
        .background_style(&WHITE)
        .draw()
        .unwrap();
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn find_command(cmd: &serde_json::Value) -> Option<String> {
    for check_command in COMMANDS {
        if cmd.get(check_command).is_some() {
            return Some(String::from(*check_command));
        }
    }
    None
}

const COMMANDS: &'static [&'static str] = &[
    // Query and Write Operation Commands
    "delete",
    "find",
    "findAndModify",
    "getMore",
    "insert",
    "resetError",
    "update",
];
