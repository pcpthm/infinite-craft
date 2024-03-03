mod helper;
mod subset;

use std::{
    collections::{BTreeSet, HashSet},
    path::PathBuf,
    time::Instant,
};

use clap::Parser;

use helper::Helper;
use infinite_craft::{
    get_path,
    set_enum::{collect_new_pairs, enum_set},
    NameMap, Recipe,
};

use crate::subset::{get_max_removal, get_unreachable};

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
    res.sort();
    res.dedup();
    Ok(res)
}

fn collect_recipe_next(
    d: usize,
    init: &[u32],
    recipe: &mut Recipe,
    nm: &mut NameMap,
    helper: &mut Helper,
) -> anyhow::Result<()> {
    let mut new_pairs = HashSet::new();
    {
        let instant = Instant::now();
        let count = collect_new_pairs(d - 1, init, recipe, &mut new_pairs);
        let time = instant.elapsed();
        eprintln!("depth={}: {} sets in {}ms", d, count, time.as_millis());
    }

    let mut new_pairs = Vec::from_iter(new_pairs);
    new_pairs.sort();

    helper.progress_reset(new_pairs.len(), "pair")?;
    for pair in new_pairs {
        let [u, v] = pair.get();
        let result = helper.pair(nm.name(u), nm.name(v))?;
        recipe.insert(u, v, nm.intern(result));
    }
    Ok(())
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
    let mut nm = NameMap::new();

    let mut init = Vec::from_iter(args.init.iter().map(|name| nm.intern(name)));
    if let Some(path) = &args.init_file {
        init.extend(read_elements_from_file(path, &mut nm)?);
    }
    init.sort();
    init.dedup();

    let mut recipe = Recipe::new();
    let mut helper = Helper::start()?;

    for d in 1..=args.depth {
        let start = nm.len();
        collect_recipe_next(d, &init, &mut recipe, &mut nm, &mut helper)?;

        for u in start as u32..nm.len() as u32 {
            println!("{}={}", nm.name(u), d);
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

fn get_pair_all(nm: &NameMap, recipe: &mut Recipe, helper: &mut Helper) -> anyhow::Result<()> {
    helper.progress_reset(nm.items().count() * (nm.items().count() + 1) / 2, "pair")?;
    for u in nm.items() {
        for v in nm.items().filter(|&v| u >= v) {
            if let Some(w) = nm.lookup(helper.pair(nm.name(u), nm.name(v))?) {
                // println!("{}={}={}", nm.name(u), nm.name(v), nm.name(w));
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
    let set = read_elements_from_file(&args.set, &mut nm)?;

    let mut recipe = Recipe::new();
    let mut helper = Helper::start()?;
    get_pair_all(&nm, &mut recipe, &mut helper)?;

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
struct SubsetArgs {
    #[clap(long, default_values_t = default_init())]
    init: Vec<String>,

    target: PathBuf,

    extra: PathBuf,

    #[clap(long, default_value_t = 0)]
    depth: usize,

    #[clap(long, default_value_t = 20)]
    token1: usize,

    #[clap(long, default_value_t = 20)]
    token2: usize,
}

fn explore_subset(args: &SubsetArgs) -> anyhow::Result<()> {
    let mut nm = NameMap::new();
    let init = Vec::from_iter(args.init.iter().map(|name| nm.intern(name)));
    let target = read_elements_from_file(&args.target, &mut nm)?;
    let extra = read_elements_from_file(&args.extra, &mut nm)?;
    let extra = Vec::from_iter(extra.into_iter().filter(|u| !target.contains(u)));
    let ss = nm.items().collect::<Vec<_>>();

    println!("{} target, {} extra", target.len(), extra.len());

    let mut recipe = Recipe::new();
    let mut helper = Helper::start()?;
    get_pair_all(&nm, &mut recipe, &mut helper)?;

    // TODO: off-by-once? depth=1 should try (extra + all) for example.
    let mut extra_sets = vec![Vec::new()];
    for d in 1..=args.depth {
        collect_recipe_next(d, &ss, &mut recipe, &mut nm, &mut helper)?;
        enum_set(d, &ss, &recipe, &mut |queue, set| {
            for &u in queue.as_slice() {
                let mut set = Vec::from_iter(set.iter().copied().filter(|&u| !ss.contains(&u)));
                set.push(u);
                extra_sets.push(set);
            }
        })
    }
    drop(helper);

    let mut state = vec![0; nm.len()];
    let unreachable = get_unreachable(&init, &extra, &target, &mut state, &recipe);
    if !unreachable.is_empty() {
        println!("# {} targets unreachable", unreachable.len());
        for &u in &unreachable {
            println!("{}", nm.name(u));
        }
        return Ok(());
    }

    let mut min_sets: BTreeSet<Vec<u32>> = BTreeSet::new();
    for (i, ex) in extra_sets.iter().enumerate() {
        let mut sets = Vec::new();
        let target_ex = Vec::from_iter(target.iter().chain(ex).copied());
        get_max_removal(&init, &target_ex, &extra, &mut state, &recipe, &mut sets);
        for rm in sets {
            let mut set = Vec::from_iter(extra.iter().copied().filter(|&u| !rm.contains(&u)));
            set.extend_from_slice(ex);
            if min_sets.is_empty() || min_sets.first().unwrap().len() > set.len() {
                min_sets.clear();
            }
            if min_sets.is_empty() || min_sets.first().unwrap().len() == set.len() {
                min_sets.insert(set);
            }
        }
        eprint!(
            "\r{}/{}: {}, {}",
            i + 1,
            extra_sets.len(),
            min_sets.first().unwrap().len(),
            min_sets.len(),
        );
    }

    for set in &min_sets {
        let added = Vec::from_iter(set.iter().copied().filter(|&u| !extra.contains(&u)));
        let removed = Vec::from_iter(extra.iter().copied().filter(|&u| !set.contains(&u)));
        print!("# Added {}: ", added.len());
        for &u in &added {
            print!("{:?}, ", nm.name(u))
        }
        print!("Removed {}: ", removed.len());
        for &u in &removed {
            print!("{:?}, ", nm.name(u))
        }
        println!();
    }

    Ok(())
}

#[derive(clap::Parser)]
enum App {
    Enum(EnumArgs),
    Path(ReconstructPathArgs),
    Subset(SubsetArgs),
}

fn main() -> anyhow::Result<()> {
    match App::parse() {
        App::Enum(args) => set_enum(&args)?,
        App::Path(args) => reconstruct_path(&args)?,
        App::Subset(args) => explore_subset(&args)?,
    }
    Ok(())
}
