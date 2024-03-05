//! Interface for the infinite-craft.py script, which does actual API requests .

use std::{
    collections::HashMap,
    io::{BufRead, BufReader, BufWriter, Write},
    process::{Command, Stdio},
};

use anyhow::Context;
use infinite_craft::{ElementSet, Pair, SymPair};

pub struct Db {
    pair: HashMap<SymPair, u32>,
    token_count: HashMap<u32, usize>,

    child_in: Box<dyn Write>,
    child_out: Box<dyn BufRead>,
    line: String,
}

fn canon(p: Pair, elems: &ElementSet) -> SymPair {
    let [u, v] = p.get();
    SymPair::new(elems.canon(u), elems.canon(v))
}

impl Db {
    pub fn open() -> anyhow::Result<Self> {
        let mut cmd = Command::new("python3");
        cmd.arg("infinite-craft.py");
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped());
        let child = cmd.spawn().context("Failed to start helper script")?;
        Ok(Self {
            pair: HashMap::new(),
            token_count: HashMap::new(),

            child_in: Box::new(BufWriter::new(child.stdin.unwrap())),
            child_out: Box::new(BufReader::new(child.stdout.unwrap())),
            line: String::new(),
        })
    }

    pub fn get_pair(&self, p: Pair, elems: &ElementSet) -> Option<u32> {
        self.pair.get(&canon(p, elems)).copied()
    }

    pub fn get_token_count(&self, u: u32) -> Option<usize> {
        self.token_count.get(&u).copied()
    }

    pub fn pair_all(
        &mut self,
        iter: impl Iterator<Item = Pair>,
        elems: &mut ElementSet,
    ) -> anyhow::Result<()> {
        let mut to_pair = iter
            .map(|p| canon(p, elems))
            .filter(|p| !self.pair.contains_key(p))
            .collect::<Vec<_>>();
        to_pair.sort();
        to_pair.dedup();

        self.progress_reset(to_pair.len(), "pair")?;
        for p in to_pair {
            let [first, second] = p.get().map(|u| elems.name(u));
            let w = elems.intern(self.pair(first, second)?);
            self.pair.insert(p, w);
        }
        Ok(())
    }

    pub fn tokenize_all(
        &mut self,
        iter: impl Iterator<Item = u32>,
        elems: &ElementSet,
    ) -> anyhow::Result<()> {
        let mut to_tokenize = Vec::from_iter(iter.filter(|u| !self.token_count.contains_key(u)));
        to_tokenize.sort();
        to_tokenize.dedup();

        self.progress_reset(to_tokenize.len(), "tokenize")?;
        for u in to_tokenize {
            let count = self.tokenize(elems.name(u))?;
            self.token_count.insert(u, count);
        }
        Ok(())
    }

    fn progress_reset(&mut self, count: usize, desc: &str) -> anyhow::Result<()> {
        writeln!(&mut self.child_in, "progress_reset:{} {}", count, desc)?;
        self.child_in.flush()?;
        Ok(())
    }

    fn pair(&mut self, first: &str, second: &str) -> anyhow::Result<&str> {
        writeln!(&mut self.child_in, "pair:{}={}", first, second)?;
        self.child_in.flush()?;

        self.line.clear();
        self.child_out.read_line(&mut self.line)?;
        Ok(self.line.trim_end())
    }

    fn tokenize(&mut self, name: &str) -> anyhow::Result<usize> {
        writeln!(&mut self.child_in, "tokenize:{}", name)?;
        self.child_in.flush()?;

        self.line.clear();
        self.child_out.read_line(&mut self.line)?;
        Ok(self.line.trim_end().parse()?)
    }
}
