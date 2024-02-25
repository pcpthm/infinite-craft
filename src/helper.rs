use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

use anyhow::Context;

use crate::cache::PairCache;

pub struct RequestProxy {
    child_in: Box<dyn Write>,
    child_out: Box<dyn BufRead>,
}

impl RequestProxy {
    pub fn start() -> anyhow::Result<Self> {
        let child = Command::new("python3")
            .arg("infinite-craft.py")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to run helper script")?;
        Ok(Self {
            child_in: Box::new(child.stdin.unwrap()),
            child_out: Box::new(BufReader::new(child.stdout.unwrap())),
        })
    }

    pub fn get_pair<'a>(
        &mut self,
        first: &str,
        second: &str,
        line: &'a mut String,
    ) -> anyhow::Result<&'a str> {
        writeln!(&mut self.child_in, "pair:{}={}", first, second)?;
        self.child_in.flush()?;

        line.clear();
        self.child_out.read_line(line)?;

        Ok(line.trim_end())
    }

    pub fn dump_db_start(&mut self) -> anyhow::Result<()> {
        writeln!(&mut self.child_in, "dump:")?;
        self.child_in.flush()?;
        Ok(())
    }

    pub fn dump_db_next<'a>(
        &mut self,
        line: &'a mut String,
    ) -> anyhow::Result<Option<[&'a str; 3]>> {
        line.clear();
        self.child_out.read_line(line)?;

        Ok(split_line(line.trim_end()))
    }

    pub fn tokenize(&mut self, text: &str, line: &mut String) -> anyhow::Result<usize> {
        writeln!(&mut self.child_in, "tokenize:{}", text)?;
        self.child_in.flush()?;

        line.clear();
        self.child_out.read_line(line)?;
        Ok(line.trim_end().parse()?)
    }
}

fn split_line(line: &str) -> Option<[&str; 3]> {
    let (first, rest) = line.trim_end().split_once('=')?;
    let (second, result) = rest.split_once('=')?;
    Some([first, second, result])
}

pub struct DynamicCache {
    cache: PairCache,
    proxy: RequestProxy,
    tokens: Vec<u8>,
    line: String,
}

impl DynamicCache {
    pub fn new(cache: PairCache, proxy: RequestProxy) -> Self {
        Self {
            cache,
            proxy,
            tokens: Vec::new(),
            line: String::new(),
        }
    }

    pub fn populate_cache(&mut self) -> anyhow::Result<()> {
        self.proxy.dump_db_start()?;
        while let Some([first, second, result]) = self.proxy.dump_db_next(&mut self.line)? {
            let pair: [u32; 2] = [first, second].map(|name| self.cache.intern(name));
            let result_id = self.cache.intern(result);
            if let Some(existing) = self.cache.get(pair) {
                if existing != result_id {
                    let existing = self.cache.name(existing);
                    eprintln!("conflict: {}+{}={} != {}", first, second, existing, result);
                }
            }
            self.cache.insert(pair, result_id);
        }
        Ok(())
    }

    pub fn intern(&mut self, name: &str) -> u32 {
        self.cache.intern(name)
    }

    pub fn cache(&self) -> &PairCache {
        &self.cache
    }

    pub fn get(&mut self, pair: [u32; 2]) -> anyhow::Result<u32> {
        if let Some(result) = self.cache.get(pair) {
            return Ok(result);
        }

        let [first, second] = pair.map(|name| self.cache.name(name));
        let result = self.proxy.get_pair(first, second, &mut self.line)?;
        let result = self.cache.intern(result);
        self.cache.insert(pair, result);
        Ok(result)
    }

    pub fn get_token_count(&mut self, u: u32) -> anyhow::Result<usize> {
        let i = u as usize;
        if i >= self.tokens.len() || self.tokens[i] == 0 {
            self.tokens.resize(self.tokens.len().max(i + 1), 0);

            let name = self.cache.name(u);
            self.tokens[i] = u8::try_from(self.proxy.tokenize(name, &mut self.line)?)?;
        }
        Ok(self.tokens[i] as usize)
    }
}
