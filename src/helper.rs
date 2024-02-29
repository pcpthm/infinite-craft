//! Interface for the infinite-craft.py script, which does actual API requests .

use std::{
    io::{BufRead, BufReader, BufWriter, Write},
    process::{Command, Stdio},
};

use anyhow::Context;

use crate::NameMap;

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

    pub fn pair(&mut self, pair: [u32; 2], nm: &mut NameMap) -> anyhow::Result<u32> {
        let pair = [nm.name(pair[0]), nm.name(pair[1])];
        writeln!(&mut self.child_in, "pair:{}={}", pair[0], pair[1])?;
        self.child_in.flush()?;

        self.line.clear();
        self.child_out.read_line(&mut self.line)?;
        Ok(nm.intern(self.line.trim_end()))
    }

    pub fn tokenize(&mut self, id: u32, nm: &NameMap) -> anyhow::Result<usize> {
        writeln!(&mut self.child_in, "tokenize:{}", nm.name(id))?;
        self.child_in.flush()?;

        self.line.clear();
        self.child_out.read_line(&mut self.line)?;
        Ok(self.line.trim_end().parse()?)
    }
}
