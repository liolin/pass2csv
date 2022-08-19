use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{self, Command};

use serde::Serialize;
use structopt::StructOpt;
use thiserror::Error;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    #[structopt(short, long, parse(from_os_str))]
    output: PathBuf,

    #[structopt(short, long, parse(from_os_str))]
    input: PathBuf,

    #[structopt(short, long)]
    delimiter: String,
}

#[derive(Error, Debug)]
pub enum Pass2CSVError {
    #[error("Password not found")]
    NoPassword,
}

#[derive(Debug, Serialize)]
struct Entry {
    password: String,
    additional: String,
    file_path: String,
}

impl Entry {
    pub fn new(content: &str, file_path: &str, delimiter: &str) -> anyhow::Result<Entry> {
        let mut lines = content.lines();
        let password = Self::parse_password(&mut lines)?;
        let additional = Self::parse_additional(&mut lines, delimiter);

        Ok(Entry {
            password,
            file_path: file_path.to_string(),
            additional,
        })
    }

    pub fn from_file(path: &str, delimiter: &str) -> Entry {
        let output = Command::new("gpg")
            .arg("--decrypt")
            .arg(path)
            .output()
            .expect("Failed to execute command");

        if !output.status.success() {
            io::stderr().write_all(&output.stderr).unwrap();
            process::exit(output.status.code().unwrap_or(1));
        }

        let content = std::str::from_utf8(&output.stdout).unwrap().trim();
        Entry::new(content, path, delimiter).unwrap()
    }

    fn parse_additional(lines: &mut std::str::Lines, delimiter: &str) -> String {
        lines
            .map(|l| {
                let mut s = l.to_string();
                s.push_str(delimiter);
                s
            })
            .collect()
    }

    fn parse_password(lines: &mut std::str::Lines) -> anyhow::Result<String> {
        Ok(lines
            .next()
            .ok_or::<Pass2CSVError>(Pass2CSVError::NoPassword.into())?
            .to_string())
    }
}

fn main() {
    let opt = Opt::from_args();

    let output = Command::new("fd")
        .arg("-e")
        .arg("gpg")
        .arg(".")
        .arg(opt.input)
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        io::stderr()
            .write_all(&output.stderr)
            .expect("Error happended. Could not write erros to stderr");
        process::exit(output.status.code().unwrap_or(1));
    }

    let files = std::str::from_utf8(&output.stdout).expect("Output from `fd` was not valid UTF-8");
    let file = std::fs::File::create(&opt.output).expect("Coul't not create output file");
    let mut wtr = csv::Writer::from_writer(file);

    for file_path in files.lines() {
        let entry = Entry::from_file(file_path, &opt.delimiter);
        wtr.serialize(&entry).expect(&format!(
            "Could not serialize the file {}.\n{:#?}",
            file_path, entry
        ));
    }
    wtr.flush().expect(&format!(
        "Could not write the serialized entries to the file {}",
        &opt.output
            .into_os_string()
            .into_string()
            .expect("Could not convert path to UTF-8 string")
    ))
}
