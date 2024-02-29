//! Interface for the infinite-craft.py script, which does actual API requests .

use std::{
    io::{BufRead, BufReader, BufWriter, Write},
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
        let mut cmd = Command::new("python3");
        cmd.arg("infinite-craft.py");
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped());
        let child = cmd.spawn().context("Failed to start helper script")?;
        Ok(Self {
            child_in: Box::new(BufWriter::new(child.stdin.unwrap())),
            child_out: Box::new(BufReader::new(child.stdout.unwrap())),
            line: String::new(),
        })
    }

    pub fn progress_reset(&mut self, count: usize, desc: &str) -> anyhow::Result<()> {
        writeln!(&mut self.child_in, "progress_reset:{} {}", count, desc)?;
        self.child_in.flush()?;
        Ok(())
    }

    pub fn pair(&mut self, first: &str, second: &str) -> anyhow::Result<&str> {
        writeln!(&mut self.child_in, "pair:{}={}", first, second)?;
        self.child_in.flush()?;

        self.line.clear();
        self.child_out.read_line(&mut self.line)?;
        Ok(self.line.trim_end())
    }

    pub fn tokenize(&mut self, name: &str) -> anyhow::Result<usize> {
        writeln!(&mut self.child_in, "tokenize:{}", name)?;
        self.child_in.flush()?;

        self.line.clear();
        self.child_out.read_line(&mut self.line)?;
        Ok(self.line.trim_end().parse()?)
    }
}
