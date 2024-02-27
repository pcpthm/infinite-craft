//! Interface for the infinite-craft.py script, which does actual API requests .

use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

use anyhow::Context;

use crate::cache::Cache;

pub struct Helper<'a> {
    cache: &'a mut Cache,
    child_in: Box<dyn Write>,
    child_out: Box<dyn BufRead>,
    line: String,
}

impl<'a> Helper<'a> {
    pub fn start(cache: &'a mut Cache) -> anyhow::Result<Self> {
        let child = Command::new("python3")
            .arg("infinite-craft.py")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to start helper script")?;
        Ok(Self {
            cache,
            child_in: Box::new(child.stdin.unwrap()),
            child_out: Box::new(BufReader::new(child.stdout.unwrap())),
            line: String::new(),
        })
    }

    pub fn intern(&mut self, name: &str) -> u32 {
        self.cache.intern(name)
    }

    pub fn pair(&mut self, u: u32, v: u32) -> anyhow::Result<u32> {
        if let Some(w) = self.cache.pair([u, v]) {
            return Ok(w);
        }
        let pair = [self.cache.name(u), self.cache.name(v)];
        writeln!(&mut self.child_in, "pair:{}={}", pair[0], pair[1])?;
        self.child_in.flush()?;

        self.line.clear();
        self.child_out.read_line(&mut self.line)?;

        let w = self.cache.intern(self.line.trim_end());
        self.cache.insert_pair([u, v], w);
        Ok(w)
    }

    pub fn tokenize(&mut self, u: u32) -> anyhow::Result<usize> {
        if let Some(count) = self.cache.tokenize(u) {
            return Ok(count);
        }
        writeln!(&mut self.child_in, "tokenize:{}", self.cache.name(u))?;
        self.child_in.flush()?;

        self.line.clear();
        self.child_out.read_line(&mut self.line)?;

        let count = self.line.trim_end().parse()?;
        self.cache.insert_tokenize(u, count);
        Ok(count)
    }
}
