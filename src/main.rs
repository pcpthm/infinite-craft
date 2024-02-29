#![allow(clippy::new_without_default, clippy::len_without_is_empty)]

mod helper;
mod set_enum;

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    time::Instant,
};

use clap::Parser;

use helper::Helper;
use set_enum::{collect_new_pairs, BLOCKED, BLOCKED_FIRST, BLOCKED_SECOND};

const NOTHING: u32 = 0;

pub struct NameMap {
    name: Vec<String>,
    id: HashMap<String, u32>,
}

impl NameMap {
    pub fn new() -> Self {
        Self {
            name: vec!["Nothing".to_owned()],
            id: [("".to_owned(), NOTHING)].into_iter().collect(),
        }
    }

    pub fn intern(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.id.get(name) {
            return id;
        }
        let id = self.name.len() as u32;
        self.name.push(name.to_string());
        self.id.insert(name.to_owned(), id);
        id
    }

    pub fn name(&self, id: u32) -> &str {
        &self.name[id as usize]
    }

    pub fn len(&self) -> usize {
        self.name.len()
    }
}

#[derive(Parser)]
struct SetEnumArgs {
    /// Maximum depth to search.
    #[clap(long, default_value_t = 5)]
    depth: usize,

    /// Initial set of items that can be freely used.
    #[clap(long, default_values_t = ["Water", "Fire", "Wind", "Earth"].map(String::from))]
    init: Vec<String>,

    #[clap(long = "init-file")]
    init_file: Option<PathBuf>,

    /// Maximum token count of an item to expand the search from.
    #[clap(long, default_value_t = 20)]
    token1: usize,

    /// Maximum token count of an item to be tried for pairing.
    #[clap(long, default_value_t = 20)]
    token2: usize,

    /// Restrict the search to a subset of items. Initial items are always allowed.
    #[clap(long)]
    subset: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = SetEnumArgs::parse();
    let mut helper = Helper::start()?;

    let mut nm = NameMap::new();

    let mut init: Vec<u32> = args.init.iter().map(|name| nm.intern(name)).collect();
    if let Some(path) = &args.init_file {
        init.extend(std::fs::read_to_string(path)?.lines().map(|n| nm.intern(n)));
    }
    init.sort();
    init.dedup();

    let mut allowed_items = usize::MAX;
    if let Some(path) = &args.subset {
        for name in std::fs::read_to_string(path)?.lines() {
            nm.intern(name);
        }
        allowed_items = nm.len();
    }

    let mut blocked = vec![BLOCKED];
    let mut recipe = HashMap::new();

    for depth in 1..=args.depth {
        blocked.reserve(nm.len() - blocked.len());
        helper.progress_reset(nm.len() - blocked.len(), "tokenize")?;
        for i in blocked.len()..nm.len() {
            let c = helper.tokenize(nm.name(i as u32))?;
            let mut b = 0;
            b |= if allowed_items <= i { BLOCKED } else { 0 };
            b |= if args.token1 < c { BLOCKED_FIRST } else { 0 };
            b |= if args.token2 < c { BLOCKED_SECOND } else { 0 };
            blocked.push(b);
        }

        let mut new_pairs = HashSet::new();

        let instant = Instant::now();
        collect_new_pairs(depth, &init, &blocked, &recipe, &mut new_pairs);
        eprintln!("depth={}: {}ms", depth, instant.elapsed().as_millis());

        let mut new_pairs = Vec::from_iter(new_pairs.into_iter());
        new_pairs.sort();

        helper.progress_reset(new_pairs.len(), "pair")?;
        for &pair in &new_pairs {
            let [u, v] = pair.get();
            let result = helper.pair(nm.name(u), nm.name(v))?;
            recipe.insert(pair, nm.intern(result));
        }
    }

    Ok(())
}
