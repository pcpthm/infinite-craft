mod db;

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::PathBuf,
    time::Instant,
};

use clap::Parser;

use db::Db;
use infinite_craft::{
    get_path,
    set_enum::{collect_new_pairs, enum_set},
    subset::{get_max_removal, get_unreachable},
    ElementSet, Pair,
};

fn pair_all(
    pairs: &HashSet<Pair>,
    max_token: [usize; 2],
    elems: &mut ElementSet,
    recipe: &mut HashMap<Pair, u32>,
    db: &mut Db,
) -> anyhow::Result<()> {
    db.tokenize_all(pairs.iter().flat_map(|p| p.get()), elems)?;

    let pairs = Vec::from_iter(pairs.iter().copied().filter(|p| {
        let t = p.get().map(|u| db.get_token_count(u).unwrap());
        t[0] <= max_token[0] && t[1] <= max_token[1]
    }));
    db.pair_all(pairs.iter().copied(), elems)?;

    for p in pairs {
        recipe.insert(p, db.get_pair(p, elems).unwrap());
    }
    Ok(())
}

fn default_init() -> Vec<String> {
    ["Water", "Fire", "Wind", "Earth"].map(String::from).into()
}

fn read_elements_from_file(path: &PathBuf, elems: &mut ElementSet) -> anyhow::Result<Vec<u32>> {
    let mut res = Vec::new();
    for line in std::fs::read_to_string(path)?.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((_, result)) = line.rsplit_once(" -> ") {
            res.push(elems.intern(result));
        } else {
            res.push(elems.intern(line));
        }
    }
    res.sort();
    res.dedup();
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
    let mut elems = ElementSet::new();
    let mut init = Vec::from_iter(args.init.iter().map(|name| elems.intern(name)));
    if let Some(path) = &args.init_file {
        init.extend(read_elements_from_file(path, &mut elems)?);
    }
    init.sort();
    init.dedup();

    let mut recipe = HashMap::new();
    let mut db = Db::open()?;

    for d in 0..args.depth {
        let n = elems.len();
        let instant = Instant::now();
        let (pairs, count) = collect_new_pairs(d, n, &init, &recipe);
        let time = instant.elapsed();
        eprintln!("depth={}: {} sets in {}ms", d + 1, count, time.as_millis());

        let max_token = [args.token1, args.token2];
        pair_all(&pairs, max_token, &mut elems, &mut recipe, &mut db)?;

        for u in n as u32..elems.len() as u32 {
            println!("{}={}", elems.name(u), d + 1);
        }
    }

    Ok(())
}

#[derive(clap::Parser)]
struct ReconstructPathArgs {
    set: PathBuf,

    #[clap(long, default_values_t = default_init())]
    init: Vec<String>,

    #[clap(long)]
    alt: bool,
}

fn format_triple(triple: [u32; 3], elems: &ElementSet) -> String {
    let [u, v, w] = triple;
    format!("{} + {} -> {}", elems.name(u), elems.name(v), elems.name(w))
}

fn reconstruct_path(args: &ReconstructPathArgs) -> anyhow::Result<()> {
    let mut elems = ElementSet::new();
    let init = Vec::from_iter(args.init.iter().map(|name| elems.intern(name)));
    let set = read_elements_from_file(&args.set, &mut elems)?;
    let all = Vec::from_iter(elems.items());

    let mut recipe = HashMap::new();
    let mut db = Db::open()?;
    let pairs = collect_new_pairs(0, elems.len(), &all, &recipe).0;
    pair_all(&pairs, [!0, !0], &mut elems, &mut recipe, &mut db)?;

    let path = get_path(&init, &set, &recipe);
    if path.len() != set.len() {
        println!("# Incomplete ({}/{})", path.len(), set.len());
    }
    for (i, &triple) in path.iter().enumerate() {
        if i != 0 && triple[0] != path[i - 1][2] && triple[1] != path[i - 1][2] {
            println!();
        }

        if args.alt {
            for j in (0..i).rev() {
                for k in (0..j + 1).rev() {
                    let t = [path[j][2], path[k][2], path[i][2]];
                    if recipe.get(&Pair::new(t[0], t[1])) == Some(&t[2]) {
                        println!("# {}", format_triple(t, &elems));
                    }
                }
            }
        }
        println!("{}", format_triple(triple, &elems));
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
    let mut elems = ElementSet::new();
    let init = Vec::from_iter(args.init.iter().map(|name| elems.intern(name)));
    let target = read_elements_from_file(&args.target, &mut elems)?;
    let extra = read_elements_from_file(&args.extra, &mut elems)?;
    let extra = Vec::from_iter(extra.into_iter().filter(|u| !target.contains(u)));
    let superset = elems.items().collect::<Vec<_>>();

    println!("{} target, {} extra", target.len(), extra.len());

    let mut recipe = HashMap::new();
    let mut db = Db::open()?;

    for d in 0..=args.depth {
        let pairs = collect_new_pairs(d, elems.len(), &superset, &recipe).0;
        let max_token = [args.token1, args.token2];
        pair_all(&pairs, max_token, &mut elems, &mut recipe, &mut db)?;
    }

    let mut extra_sets = Vec::new();
    enum_set(elems.len(), &superset, &recipe, &mut |_, set| {
        extra_sets.push(set[superset.len()..].to_owned());
        args.depth < set.len() - superset.len() + 1
    });

    let pairs = collect_new_pairs(0, elems.len(), &superset, &recipe).0;
    pair_all(&pairs, [!0, !0], &mut elems, &mut recipe, &mut db)?;
    drop(db);

    let mut remain = vec![false; elems.len()];
    let unreachable = get_unreachable(&init, &extra, &target, &mut remain, &recipe);
    if !unreachable.is_empty() {
        println!("# {} targets unreachable", unreachable.len());
        for &u in &unreachable {
            println!("{}", elems.name(u));
        }
        return Ok(());
    }

    let mut min_sets: BTreeSet<Vec<u32>> = BTreeSet::new();
    for (i, ex) in extra_sets.iter().enumerate() {
        let mut sets = Vec::new();
        let target_ex = Vec::from_iter(target.iter().chain(ex).copied());
        get_max_removal(&init, &target_ex, &extra, &mut remain, &recipe, &mut sets);
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
            print!("{:?}, ", elems.name(u))
        }
        print!("Removed {}: ", removed.len());
        for &u in &removed {
            print!("{:?}, ", elems.name(u))
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
