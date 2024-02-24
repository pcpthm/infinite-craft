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
        writeln!(&mut self.child_in, "{}={}", first, second)?;
        self.child_in.flush()?;

        line.clear();
        self.child_out.read_line(line)?;

        Ok(line.trim_end())
    }

    pub fn dump_db_start(&mut self) -> anyhow::Result<()> {
        writeln!(&mut self.child_in, "==dump")?;
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
}

fn split_line(line: &str) -> Option<[&str; 3]> {
    let (first, rest) = line.trim_end().split_once('=')?;
    let (second, result) = rest.split_once('=')?;
    Some([first, second, result])
}

pub struct DynamicCache {
    cache: PairCache,
    proxy: RequestProxy,
    line: String,
}

impl DynamicCache {
    pub fn new(cache: PairCache, proxy: RequestProxy) -> Self {
        Self {
            cache,
            proxy,
            line: String::new(),
        }
    }

    pub fn populate_cache(&mut self) -> anyhow::Result<()> {
        self.proxy.dump_db_start()?;
        while let Some([first, second, result]) = self.proxy.dump_db_next(&mut self.line)? {
            insert_check_conflict(first, second, result, &mut self.cache);
        }
        Ok(())
    }

    pub fn num_items(&self) -> usize {
        self.cache.num_items()
    }

    pub fn intern(&mut self, name: &str) -> u32 {
        self.cache.intern(name)
    }

    #[inline]
    pub fn cache(&self) -> &PairCache {
        &self.cache
    }

    #[inline]
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
}

fn insert_check_conflict(first: &str, second: &str, result: &str, cache: &mut PairCache) {
    let pair = [first, second].map(|name| cache.intern(name));
    let result_id = cache.intern(result);
    if let Some(existing) = cache.get(pair) {
        if existing != result_id {
            let existing = cache.name(existing);
            eprintln!("conflict: {}+{}={} != {}", first, second, existing, result);
        }
        return;
    }
    cache.insert(pair, result_id);
}
