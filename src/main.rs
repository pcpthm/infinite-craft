mod helper;

use std::{
    collections::{hash_map::Entry, HashMap},
    path::PathBuf,
    time::Instant,
};

use clap::Parser;
use helper::Helper;

const UNKNOWN: u32 = 0;
const NOTHING: u32 = 1;

pub struct NameMap {
    name: Vec<String>,
    id: HashMap<String, u32>,
}

impl NameMap {
    pub fn new() -> Self {
        Self {
            name: vec![String::new(), "Nothing".to_owned()],
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

const BLOCKED: u8 = 1;
const BLOCKED_FIRST: u8 = 2;
const BLOCKED_SECOND: u8 = 4;
const BLOCKED_INIT: u8 = 8;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SymPair(u64);

impl SymPair {
    pub fn new(u: u32, v: u32) -> Self {
        let (u, v) = if u > v { (v, u) } else { (u, v) };
        Self((u as u64) << 32 | v as u64)
    }

    pub fn get(self) -> [u32; 2] {
        [(self.0 >> 32) as u32, self.0 as u32]
    }
}

fn collect_next_pairs(
    rem_depth: usize,
    qh: usize,
    queue: &mut Vec<u32>,
    blocked: &mut [u8],
    to_pair: &mut Vec<u32>,
    recipe: &mut HashMap<SymPair, u32>,
) {
    let qt = queue.len();
    for i in qh..qt {
        let u = queue[i];
        if blocked[u as usize] & (BLOCKED_SECOND | BLOCKED_INIT) == 0 {
            to_pair.push(u);
        }
        if blocked[u as usize] & BLOCKED_FIRST == 0 {
            for &v in to_pair.iter() {
                match recipe.entry(SymPair::new(u, v)) {
                    Entry::Occupied(entry) => {
                        let w = *entry.get();
                        if blocked[w as usize] & BLOCKED == 0 {
                            blocked[w as usize] |= BLOCKED;
                            queue.push(w);
                        }
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(UNKNOWN);
                    }
                }
            }
        }
        if rem_depth > 0 {
            collect_next_pairs(rem_depth - 1, i + 1, queue, blocked, to_pair, recipe);
        }
        for &w in &queue[qt..] {
            blocked[w as usize] &= !BLOCKED;
        }
        queue.truncate(qt);
        if blocked[u as usize] & (BLOCKED_SECOND | BLOCKED_INIT) == 0 {
            to_pair.pop();
        }
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

    let mut blocked = vec![BLOCKED, BLOCKED];
    let mut recipe = HashMap::new();

    for depth in 1..=args.depth {
        blocked.reserve(nm.len() - blocked.len());
        helper.progress_reset(nm.len() - blocked.len(), "tokenize")?;
        for i in blocked.len()..nm.len() {
            let c = helper.tokenize(i as u32, &nm)?;
            let mut b = 0;
            b |= if allowed_items <= i { BLOCKED } else { 0 };
            b |= if args.token1 < c { BLOCKED_FIRST } else { 0 };
            b |= if args.token2 < c { BLOCKED_SECOND } else { 0 };
            blocked.push(b);
        }

        let mut to_pair = Vec::new();
        for &u in &init {
            if blocked[u as usize] & (BLOCKED_SECOND & BLOCKED_INIT) == 0 {
                blocked[u as usize] |= BLOCKED_INIT;
                to_pair.push(u);
            }
        }

        let instant = Instant::now();
        let mut queue = Vec::new();
        for &u in &init {
            if blocked[u as usize] & BLOCKED_FIRST == 0 {
                queue.push(u);
                collect_next_pairs(
                    depth - 1,
                    0,
                    &mut queue,
                    &mut blocked,
                    &mut to_pair,
                    &mut recipe,
                );
                queue.pop();
            }
        }
        eprintln!("depth={}: {}ms", depth, instant.elapsed().as_millis());

        for &u in &init {
            blocked[u as usize] &= !BLOCKED_INIT;
        }

        let mut new_pairs = Vec::new();
        for (&pair, &result) in &recipe {
            if result == UNKNOWN {
                new_pairs.push(pair);
            }
        }
        new_pairs.sort();

        helper.progress_reset(new_pairs.len(), "pair")?;
        for &pair in &new_pairs {
            let result = helper.pair(pair.get(), &mut nm)?;
            recipe.insert(pair, result);
        }
    }

    Ok(())
}
