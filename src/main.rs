mod cache;
mod helper;

use std::{
    fs::File,
    io::{BufWriter, Write},
    str::FromStr,
    time::Instant,
};

use anyhow::Context;
use cache::{PairCache, NOTHING};
use helper::{DynamicCache, RequestProxy};

struct Enumeration {
    init_len: usize,
    set: Vec<u32>,
    max_set_len: usize,
    queue: Vec<u32>,
    blocked: Vec<bool>,

    count: usize,
    min_sets: Vec<Vec<Vec<u32>>>,
}

impl Enumeration {
    pub fn new() -> Self {
        Self {
            init_len: 0,
            set: Vec::new(),
            max_set_len: 0,
            queue: Vec::new(),
            blocked: Vec::new(),
            count: 0,
            min_sets: Vec::new(),
        }
    }

    pub fn run(&mut self, init: &[u32], max_depth: usize, recipe: &mut DynamicCache) {
        self.init_len = init.len();
        self.max_set_len = init.len() + max_depth;

        self.set.clear();
        self.set.extend_from_slice(init);

        for &u in init.iter() {
            self.add_set(0, u);
        }

        for (i, &u) in init.iter().enumerate() {
            for &v in &init[i..] {
                let w = recipe.get([u, v]).expect("couldn't get recipe");
                if w == NOTHING || w == u || w == v {
                    continue;
                }
                self.blocked.resize(recipe.num_items(), false);
                if !std::mem::replace(&mut self.blocked[w as usize], true) {
                    self.queue.push(w);
                }
            }
        }
        self.dfs(0, recipe);

        for &w in &self.queue {
            self.blocked[w as usize] = false;
        }
        self.queue.clear();
    }

    fn dfs(&mut self, qh: usize, recipe: &mut DynamicCache) {
        let qt: usize = self.queue.len();
        for i in qh..qt {
            let u = self.queue[i];
            self.set.push(u);

            if self.set.len() == self.max_set_len {
                self.add_set(self.set.len() - self.init_len, u);
            }

            if self.set.len() < self.max_set_len {
                for &v in &self.set {
                    let w = recipe.get([u, v]).expect("couldn't get recipe");
                    if w == NOTHING || w == u || w == v {
                        continue;
                    }
                    self.blocked.resize(recipe.num_items(), false);
                    if !std::mem::replace(&mut self.blocked[w as usize], true) {
                        self.queue.push(w);
                    }
                }
                self.dfs(i + 1, recipe);

                for &w in &self.queue[qt..] {
                    self.blocked[w as usize] = false;
                }
                self.queue.truncate(qt);
            }
            self.set.pop();
        }
    }

    fn add_set(&mut self, depth: usize, u: u32) {
        self.count += 1;

        if self.min_sets.len() <= u as usize {
            self.min_sets.resize_with(u as usize + 1, Vec::new);
        }
        let sets = &mut self.min_sets[u as usize];
        if sets.is_empty() || depth < sets[0].len() {
            sets.clear();
        }
        if sets.is_empty() || depth == sets[0].len() {
            sets.push(self.set[self.init_len..].to_vec());
        }
    }
}

fn get_all_min_sets(
    init: &[&str],
    max_depth: usize,
    recipe: &mut DynamicCache,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    let init: Vec<_> = init.iter().map(|name| recipe.intern(name)).collect();
    let mut enumeration = Enumeration::new();

    for depth in 1..=max_depth {
        let instant = Instant::now();
        let last_count = enumeration.count;
        enumeration.run(&init, depth, recipe);
        eprintln!(
            "depth={}: {} sets enumerated in {}ms",
            depth,
            enumeration.count - last_count,
            instant.elapsed().as_millis()
        );

        let cache = recipe.cache();
        for (u, sets) in (0u32..).zip(&enumeration.min_sets).skip(1) {
            if sets.is_empty() || sets[0].len() != depth {
                continue;
            }
            writeln!(out, "{}={}={}", cache.name(u), sets[0].len(), sets.len())?;
            for set in sets {
                for (i, &v) in set.iter().enumerate() {
                    if i != 0 {
                        write!(out, "=")?;
                    }
                    write!(out, "{}", cache.name(v))?;
                }
                writeln!(out)?;
            }
            writeln!(out)?;
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    match std::env::args().nth(1).as_deref() {
        None => eprintln!("Subcommand expected"),
        Some("depth") => {
            let max_depth = usize::from_str(&std::env::args().nth(2).context("depth expected")?)?;

            let mut recipe = DynamicCache::new(PairCache::new(), RequestProxy::start()?);
            recipe.populate_cache()?;

            let init = ["Water", "Fire", "Wind", "Earth"];
            let mut out = BufWriter::new(File::create("depth.log")?);
            get_all_min_sets(&init, max_depth, &mut recipe, &mut out)?;
        }
        _ => eprintln!("Unknown subcommand"),
    };
    Ok(())
}
