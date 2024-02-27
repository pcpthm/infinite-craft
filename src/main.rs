mod cache;
mod enumeration;
mod helper;

use std::path::PathBuf;

use cache::{Cache, NOTHING};
use enumeration::SetEnumeration;
use helper::Helper;

struct SearchArgs<'a> {
    init_set: Vec<&'a str>,
    subset_of: Vec<&'a str>,

    max_token_first: usize,
    max_token_second: usize,

    alloc_size: usize,
}

fn enqueue_next(
    depth: usize,
    u: u32,
    args: &SearchArgs,
    iter: &mut SetEnumeration,
    to_pair: &mut Vec<(usize, u32)>,
    helper: &mut Helper,
) -> anyhow::Result<()> {
    let token_count = helper.tokenize(u)?;
    if token_count <= args.max_token_second {
        to_pair.push((depth, u));
    }
    if token_count <= args.max_token_first {
        for &(_, v) in to_pair.iter() {
            iter.enqueue(helper.pair(u, v)?);
        }
    }
    Ok(())
}

fn enumerate_sets(max_depth: usize, args: &SearchArgs) -> anyhow::Result<()> {
    let mut cache = Cache::new();
    let mut helper = Helper::start(&mut cache)?;

    let mut blocked = vec![false; args.alloc_size];
    blocked[NOTHING as usize] = true;

    if !args.subset_of.is_empty() {
        let mut allowed = vec![false; args.alloc_size];
        for &name in args.init_set.iter().chain(&args.subset_of) {
            allowed[helper.intern(name) as usize] = true;
        }
        blocked.iter_mut().zip(allowed).for_each(|(b, a)| *b |= !a);
    }

    let mut iter = SetEnumeration::new(max_depth, blocked);
    let mut to_pair = Vec::with_capacity(args.init_set.len() + max_depth);

    for &name in &args.init_set {
        let u = helper.intern(name);
        enqueue_next(0, u, args, &mut iter, &mut to_pair, &mut helper)?;
    }

    let mut count = vec![0u64; max_depth + 1];
    count[0] += 1;

    let mut set = Vec::with_capacity(max_depth);
    while let Some(u) = iter.next(&mut set) {
        let depth = set.len();
        while !to_pair.is_empty() && depth <= to_pair.last().unwrap().0 {
            to_pair.pop();
        }
        if depth < max_depth {
            enqueue_next(depth, u, args, &mut iter, &mut to_pair, &mut helper)?;
        }
        count[depth] += 1;

        if depth + 7 <= max_depth {
            eprint!("progress: {:?}\r", count);
        }
    }

    for (depth, &count) in count.iter().enumerate() {
        println!("depth={}: {}", depth, count);
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let matches = clap::Command::new("enum")
        .arg(
            clap::arg!(--"depth" <DEPTH> "Maximum depth to search.")
                .value_parser(clap::value_parser!(usize))
                .default_value("5"),
        )
        .arg(
            clap::arg!(--"init" <ITEM>... "Initial set of items that can be freely used.")
                .num_args(0..)
                .default_values(["Water", "Fire", "Wind", "Earth"]),
        )
        .arg(
            clap::arg!(--"init-file" <FILE> "Initial set of items read from file, appended to items specified by --init flag.")
                .value_parser(clap::value_parser!(PathBuf))
        )
        .arg(
            clap::arg!(--"token-first" <COUNT> "Maximum token count of an item to expand the search from.")
                .value_parser(clap::value_parser!(usize))
                .default_value("20"),
        )
        .arg(
            clap::arg!(--"token-second" <COUNT> "Maximum token count of an item to be tried for pairing.")
                .value_parser(clap::value_parser!(usize))
                .default_value("20"),
        )
        .arg(
            clap::arg!(--"subset-file" <FILE> "Restrict the search to a subset of items. Initial items are always allowed.")
            .value_parser(clap::value_parser!(PathBuf))
        )
        .arg(
            clap::arg!(--"alloc-size" <INTEGER> "Maximum number of items that can be seen during the search. If it is too small, the program will exit by error.")
            .value_parser(clap::value_parser!(usize))
            .default_value("10000000")
        )
        .get_matches();

    let max_depth = *matches.get_one("depth").unwrap();

    let mut init_set = Vec::from_iter(matches.get_many("init").unwrap().map(String::as_ref));
    let init_file;
    if let Some(path) = matches.get_one::<PathBuf>("init-file") {
        init_file = std::fs::read_to_string(path)?;
        init_set.extend(init_file.lines());
    }

    let mut subset_of = Vec::new();
    let subset_file;
    if let Some(path) = matches.get_one::<PathBuf>("subset-file") {
        subset_file = std::fs::read_to_string(path)?;
        subset_of.extend(subset_file.lines());
    }

    let args = SearchArgs {
        init_set,
        subset_of,
        max_token_first: *matches.get_one("token-first").unwrap(),
        max_token_second: *matches.get_one("token-second").unwrap(),
        alloc_size: *matches.get_one("alloc-size").unwrap(),
    };
    enumerate_sets(max_depth, &args)?;
    Ok(())
}
