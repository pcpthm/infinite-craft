mod helper;

use std::{collections::HashSet, path::PathBuf, time::Instant};

use clap::Parser;

use helper::Helper;
use infinite_craft::{
    get_path,
    set_enum::{collect_new_pairs, BLOCKED, BLOCKED_FIRST, BLOCKED_SECOND},
    NameMap, Recipe, SymPair,
};

fn default_init() -> Vec<String> {
    ["Water", "Fire", "Wind", "Earth"].map(String::from).into()
}

fn read_elements_from_file(path: &PathBuf, nm: &mut NameMap) -> anyhow::Result<Vec<u32>> {
    let mut res = Vec::new();
    for line in std::fs::read_to_string(path)?.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((_, result)) = line.rsplit_once(" -> ") {
            res.push(nm.intern(result));
        } else {
            res.push(nm.intern(line));
        }
    }
    Ok(res)
}

#[derive(Parser)]
struct EnumArgs {
    /// Maximum depth to search.
    #[clap(long, default_value_t = 5)]
    depth: usize,

    /// Initial set of items that can be freely used.
    #[clap(long, default_values_t = default_init())]
    init: Vec<String>,

    #[clap(long = "init-file")]
    init_file: Option<PathBuf>,

    /// Maximum token count of an item to expand the search from.
    #[clap(long, default_value_t = 20)]
    token1: usize,

    /// Maximum token count of an item to be tried for pairing.
    #[clap(long, default_value_t = 20)]
    token2: usize,
}

fn set_enum(args: &EnumArgs) -> anyhow::Result<()> {
    let mut helper = Helper::start()?;

    let mut nm = NameMap::new();

    let mut init: Vec<u32> = args.init.iter().map(|name| nm.intern(name)).collect();
    if let Some(path) = &args.init_file {
        init.extend(read_elements_from_file(path, &mut nm)?);
    }
    init.sort();
    init.dedup();

    let mut blocked = vec![BLOCKED];
    let mut recipe = Recipe::new();

    for depth in 1..=args.depth {
        blocked.reserve(nm.len() - blocked.len());
        helper.progress_reset(nm.len() - blocked.len(), "tokenize")?;
        for i in blocked.len()..nm.len() {
            let c = helper.tokenize(nm.name(i as u32))?;
            let mut b = 0;
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
        for [u, v] in new_pairs.iter().map(|p| p.get()) {
            let result = helper.pair(nm.name(u), nm.name(v))?;
            recipe.insert(u, v, nm.intern(result));
        }
    }

    Ok(())
}

#[derive(clap::Parser)]
struct ReconstructPathArgs {
    set: PathBuf,

    #[clap(long, default_values_t = default_init())]
    init: Vec<String>,
}

fn get_pair_all(nm: &NameMap, recipe: &mut Recipe) -> anyhow::Result<()> {
    let mut helper = Helper::start()?;
    helper.progress_reset(nm.items().count() * (nm.items().count() + 1) / 2, "pair")?;
    for u in nm.items() {
        for v in nm.items().filter(|&v| u >= v) {
            if let Some(w) = nm.lookup(helper.pair(nm.name(u), nm.name(v))?) {
                recipe.insert(u, v, w);
            }
        }
    }
    Ok(())
}

fn format_triple(triple: [u32; 3], nm: &NameMap) -> String {
    let [u, v, w] = triple;
    format!("{} + {} -> {}", nm.name(u), nm.name(v), nm.name(w))
}

fn reconstruct_path(args: &ReconstructPathArgs) -> anyhow::Result<()> {
    let mut nm = NameMap::new();
    let init = Vec::from_iter(args.init.iter().map(|name| nm.intern(name)));
    let mut set = read_elements_from_file(&args.set, &mut nm)?;
    set.sort();
    set.dedup();

    let mut recipe = Recipe::new();
    get_pair_all(&nm, &mut recipe)?;

    let path = get_path(&init, &set, &recipe);
    if path.len() != set.len() {
        println!("# Incomplete ({}/{})", path.len(), set.len());
    }
    for t in path {
        println!("{}", format_triple(t, &nm));
    }
    Ok(())
}

#[derive(clap::Parser)]
struct MinimizeArgs {
    #[clap(long, default_values_t = default_init())]
    init: Vec<String>,

    target: PathBuf,

    extra: PathBuf,
}

const VISITED: u8 = 1;
const TARGET: u8 = 2;
const REMOVED: u8 = 4;

fn closure_reachable(queue: &mut Vec<u32>, state: &mut [u8], recipe: &Recipe) {
    let mut qh = 0;
    while qh < queue.len() {
        let u = queue[qh];
        qh += 1;
        for i in 0..qh {
            if let Some(w) = recipe.get(u, queue[i]) {
                if state[w as usize] & (VISITED | REMOVED) == 0 {
                    state[w as usize] |= VISITED;
                    queue.push(w);
                }
            }
        }
    }
}

fn is_target_reachable(queue: &mut Vec<u32>, state: &mut [u8], recipe: &Recipe) -> bool {
    let pqt = queue.len();
    closure_reachable(queue, state, recipe);
    let reachable = state.iter().all(|&s| s & TARGET == 0 || s & VISITED != 0);
    for &u in &queue[pqt..] {
        state[u as usize] &= !VISITED;
    }
    queue.truncate(pqt);
    reachable
}

fn get_set(state: &[u8]) -> Vec<u32> {
    (0u32..state.len() as u32)
        .filter(|&u| state[u as usize] & (VISITED | REMOVED) == 0)
        .collect()
}

fn minimize(args: &MinimizeArgs) -> anyhow::Result<()> {
    let mut nm = NameMap::new();
    let init = Vec::from_iter(args.init.iter().map(|name| nm.intern(name)));
    let target = read_elements_from_file(&args.target, &mut nm)?;
    let _ = read_elements_from_file(&args.extra, &mut nm)?;

    let mut recipe = Recipe::new();
    get_pair_all(&nm, &mut recipe)?;

    let mut queue = init.to_owned();
    let mut state = vec![0; nm.len()];
    state[0] = VISITED;
    for &u in &init {
        state[u as usize] |= VISITED;
    }
    for &u in &target {
        state[u as usize] |= TARGET;
    }

    if !is_target_reachable(&mut queue, &mut state, &recipe) {
        closure_reachable(&mut queue, &mut state, &recipe);
        for &u in target.iter().filter(|&&u| state[u as usize] & VISITED == 0) {
            println!("Unreachable: {}", nm.name(u));
        }
        return Ok(());
    }

    let mut pair_set: HashSet<_> = {
        let set = &get_set(&state);
        let path = get_path(&init, set, &recipe);
        path.into_iter().map(|t| SymPair::new(t[0], t[1])).collect()
    };

    let mut removed = vec![];
    let mut stack = vec![(0, true)];
    while let Some((mut i, lex_maximal)) = stack.pop() {
        while i < state.len() && state[i] != 0 {
            i += 1;
        }
        if i >= state.len() {
            if lex_maximal {
                print!("# Can remove:");
                for &i in &removed {
                    print!(" {:?}", nm.name(i as u32));
                }
                println!(" ({})", removed.len());

                for t in get_path(&init, &get_set(&state), &recipe) {
                    if pair_set.insert(SymPair::new(t[0], t[1])) {
                        println!("{}", format_triple(t, &nm));
                    }
                }
                println!();
            }
            if let Some(i) = removed.pop() {
                state[i] &= !REMOVED;
            }
            continue;
        }
        state[i] |= REMOVED;
        if is_target_reachable(&mut queue, &mut state, &recipe) {
            stack.push((i + 1, false));
            stack.push((i + 1, true));
            removed.push(i);
        } else {
            state[i] &= !REMOVED;
            stack.push((i + 1, lex_maximal));
        }
    }
    Ok(())
}

#[derive(clap::Parser)]
enum App {
    Enum(EnumArgs),
    Path(ReconstructPathArgs),
    Minimize(MinimizeArgs),
}

fn main() -> anyhow::Result<()> {
    match App::parse() {
        App::Enum(args) => set_enum(&args)?,
        App::Path(args) => reconstruct_path(&args)?,
        App::Minimize(args) => minimize(&args)?,
    }
    Ok(())
}
