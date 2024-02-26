//! Interface for the infinite-craft.py script, which does actual API requests .

use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

use anyhow::Context;

pub struct Helper {
    child_in: Box<dyn Write>,
    child_out: Box<dyn BufRead>,
    line: String,
}

impl Helper {
    pub fn start() -> anyhow::Result<Self> {
        let child = Command::new("python3")
            .arg("infinite-craft.py")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to start helper script")?;
        Ok(Self {
            child_in: Box::new(child.stdin.unwrap()),
            child_out: Box::new(BufReader::new(child.stdout.unwrap())),
            line: String::new(),
        })
    }

    fn read_line(&mut self) -> anyhow::Result<&str> {
        self.line.clear();
        self.child_out.read_line(&mut self.line)?;
        Ok(self.line.trim_end())
    }

    pub fn pair<'a>(&'a mut self, pair: [&str; 2]) -> anyhow::Result<&'a str> {
        writeln!(&mut self.child_in, "pair:{}={}", pair[0], pair[1])?;
        self.child_in.flush()?;
        self.read_line()
    }

    pub fn tokenize(&mut self, name: &str) -> anyhow::Result<usize> {
        writeln!(&mut self.child_in, "tokenize:{}", name)?;
        self.child_in.flush()?;
        Ok(self.read_line()?.parse()?)
    }
}
