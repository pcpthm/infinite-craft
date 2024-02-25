mod cache;
mod enumeration;
mod helper;

use std::{str::FromStr, time::Instant};

use anyhow::Context;
use cache::PairCache;
use helper::{DynamicCache, RequestProxy};

use crate::{cache::NOTHING, enumeration::HyperpathIter};

fn get_all_min_sets(init: &[u32], max_depth: usize, cache: &PairCache) -> anyhow::Result<()> {
    let mut set = Vec::with_capacity(init.len() + max_depth + 1);
    set.extend_from_slice(init);

    let mut min_sets: Vec<Vec<Vec<u32>>> = vec![Vec::new(); cache.num_items()];
    for &u in init {
        min_sets[u as usize].push(Vec::new());
    }

    for depth in 1..=max_depth {
        let instant = Instant::now();

        let mut blocked = vec![false; cache.num_items()];
        blocked[NOTHING as usize] = true;

        let mut iter = HyperpathIter::new(depth, blocked);
        for &u in &set {
            for &v in &set {
                if let Some(w) = cache.get([u, v]) {
                    iter.enqueue(w);
                }
            }
        }

        let mut count = 0;
        while let Some(u) = iter.next(&mut set) {
            if set.len() < init.len() + depth {
                for &v in &set {
                    if let Some(w) = cache.get([u, v]) {
                        iter.enqueue(w);
                    }
                }
            } else {
                let set = &set[init.len()..];
                let sets = &mut min_sets[u as usize];
                if !sets.is_empty() && set.len() < sets[0].len() {
                    sets.clear();
                }
                if sets.is_empty() || set.len() == sets[0].len() {
                    sets.push(set.to_vec());
                }
                count += 1;
            }
        }

        eprintln!(
            "depth={}: {} sets enumerated in {}ms",
            depth,
            count,
            instant.elapsed().as_millis()
        );

        for (u, sets) in (0u32..).zip(&min_sets).skip(1) {
            if sets.is_empty() || sets[0].len() != depth {
                continue;
            }
            println!("{}={}={}", cache.name(u), sets[0].len(), sets.len());
            for set in sets {
                for (i, &v) in set.iter().enumerate() {
                    if i != 0 {
                        print!("=");
                    }
                    print!("{}", cache.name(v));
                }
                println!();
            }
            println!();
        }
    }
    Ok(())
}

fn enumerate_dynamic(
    init: &[u32],
    depth: usize,
    max_tokens: usize,
    recipe: &mut DynamicCache,
) -> anyhow::Result<()> {
    let max_num_items = recipe.cache().num_items() + 1e6 as usize;

    let mut blocked = vec![false; max_num_items];
    blocked[NOTHING as usize] = true;
    let mut iter = HyperpathIter::new(depth, blocked);

    let mut set = Vec::with_capacity(init.len() + depth + 1);
    set.extend_from_slice(init);

    for &u in &set {
        for &v in &set {
            iter.enqueue(recipe.get([u, v])?);
        }
    }

    let mut count = 0;
    while let Some(u) = iter.next(&mut set) {
        if set.len() < init.len() + depth {
            for &v in &set {
                let w = recipe.get([u, v])?;
                if w == NOTHING || w == u || w == v {
                    continue;
                }
                if recipe.get_token_count(w)? <= max_tokens {
                    iter.enqueue(w);
                }
            }
        }

        count += 1;
    }
    eprintln!("depth={}: count={}", depth, count);

    Ok(())
}

fn main() -> anyhow::Result<()> {
    match std::env::args().nth(1).as_deref() {
        None => eprintln!("Subcommand expected"),
        Some("depth") => {
            let mut recipe = DynamicCache::new(PairCache::new(), RequestProxy::start()?);
            recipe.populate_cache()?;

            let init = ["Water", "Fire", "Wind", "Earth"];
            let init: Vec<_> = init.iter().map(|name| recipe.intern(name)).collect();

            let max_depth = usize::from_str(&std::env::args().nth(2).context("depth expected")?)?;
            get_all_min_sets(&init, max_depth, recipe.cache())?;
        }
        Some("depth-dynamic") => {
            let mut recipe = DynamicCache::new(PairCache::new(), RequestProxy::start()?);
            recipe.populate_cache()?;

            let init = ["Water", "Fire", "Wind", "Earth"];
            let init: Vec<_> = init.iter().map(|name| recipe.intern(name)).collect();

            let max_depth = usize::from_str(&std::env::args().nth(2).context("depth expected")?)?;
            let max_tokens = usize::from_str(&std::env::args().nth(3).context("max_tokens")?)?;

            for depth in 1..=max_depth {
                enumerate_dynamic(&init, depth, max_tokens, &mut recipe)?;
            }
        }
        _ => eprintln!("Unknown subcommand"),
    };
    Ok(())
}
