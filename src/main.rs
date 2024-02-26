mod cache;
mod enumeration;
mod helper;

use cache::{Cache, NOTHING};
use enumeration::SetEnumeration;

use helper::Helper;

fn get_pair(u: u32, v: u32, helper: &mut Helper, cache: &mut Cache) -> anyhow::Result<u32> {
    Ok(match cache.pair([u, v]) {
        Some(w) => w,
        None => {
            let result = helper.pair([cache.name(u), cache.name(v)])?;
            let w = cache.intern(result);
            cache.insert_pair([u, v], w);
            w
        }
    })
}

fn get_tokenize(u: u32, helper: &mut Helper, cache: &mut Cache) -> anyhow::Result<usize> {
    Ok(match cache.tokenize(u) {
        Some(count) => count,
        None => {
            let count = helper.tokenize(cache.name(u))?;
            cache.insert_tokenize(u, count);
            count
        }
    })
}

fn depth_search(max_depth: usize, frontier_token: usize, init_set: &[&str]) -> anyhow::Result<()> {
    let mut helper = Helper::start()?;
    let mut cache = Cache::new();

    let n = 1e7 as usize;

    let mut blocked = vec![false; n];
    blocked[NOTHING as usize] = true;

    let mut iter = SetEnumeration::new(max_depth, blocked);
    let mut path = Vec::with_capacity(init_set.len() + max_depth);

    for name in init_set {
        let u = cache.intern(name);
        path.push(u);
        for &v in &path {
            iter.enqueue(get_pair(u, v, &mut helper, &mut cache)?);
        }
    }

    let mut dist = vec![usize::MAX; n];
    let mut count = vec![0; max_depth + 1];
    for &u in &path {
        dist[u as usize] = 0;
        count[0] += 1;
    }

    while let Some(u) = iter.next(&mut path) {
        let depth = path.len() - init_set.len();
        if depth < dist[u as usize] {
            dist[u as usize] = depth;
        }
        count[depth] += 1;

        if depth < max_depth {
            let token_count = get_tokenize(u, &mut helper, &mut cache)?;
            if token_count <= frontier_token {
                for &v in &path {
                    iter.enqueue(get_pair(u, v, &mut helper, &mut cache)?);
                }
            }
        }
    }

    for (depth, &count) in count.iter().enumerate() {
        println!("{}: {}", depth, count);
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let matches = clap::Command::new("depth-search")
        .arg(
            clap::arg!(--depth <DEPTH> "maximum depth to search")
                .value_parser(clap::value_parser!(usize))
                .default_value("5"),
        )
        .arg(
            clap::arg!(--frontier-token <COUNT> "maximum token count of a frontier item")
                .value_parser(clap::value_parser!(usize))
                .default_value("30"),
        )
        .arg(
            clap::arg!(--init <ITEM>... "initial set of items that can be freely used")
                .default_values(["Water", "Fire", "Wind", "Earth"]),
        )
        .get_matches();

    depth_search(
        *matches.get_one("depth").unwrap(),
        *matches.get_one("frontier-token").unwrap(),
        &Vec::from_iter(matches.get_many("init").unwrap().map(String::as_ref)),
    )?;
    Ok(())
}
