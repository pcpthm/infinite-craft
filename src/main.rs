use std::{error::Error, time::Instant};

use infinite_craft::{find_path, search::Search, Recipe, RecipeMap};

fn main() -> Result<(), Box<dyn Error>> {
    let rm =
        RecipeMap::from_serialized(bincode::deserialize(&std::fs::read("recipe-map.bincode")?)?);

    println!("{} items, {} recipes", rm.num_items(), rm.num_recipes());

    let max_count = usize::MAX;
    let max_card = 300;
    println!("max_width={}, max_card={}", max_count, max_card);

    let mut search = Search::new(rm.num_items(), max_count, max_card);
    let graph = rm.make_graph();

    let source_names = ["Water", "Fire", "Wind", "Earth"];
    let source = source_names
        .into_iter()
        .map(|name| name_to_id(name, &rm))
        .collect::<Result<Vec<_>, _>>()?;

    {
        let instant = Instant::now();
        search.search_from_source(&source, &graph);
        println!("Search took {}ms", instant.elapsed().as_millis());
    }

    let reached: Vec<_> = rm.items().filter(|&u| search.reached(u)).collect();
    println!(
        "Reached {}/{}, Sum card: {}",
        reached.len(),
        rm.num_items(),
        reached.iter().map(|&u| search.min_card(u)).sum::<usize>()
    );

    compare_against_optimal(&search, &rm)?;

    // for &target_name in &["Human", "Pencil", "Joint"] {
    for target_name in ('A'..='Z').map(|a| &*a.to_string().leak()) {
        let target = name_to_id(target_name, &rm)?;

        println!(
            "{} paths of card={} found for {:?} from {:?}",
            search.sets(target).len(),
            search.min_card(target),
            target_name,
            &source_names,
        );
        for set in search.sets(target).iter() {
            let mut set = set.to_vec();
            set.sort_by_key(|&u| search.min_card(u));
            let path = find_path(&source, &set, &rm).unwrap();
            println!("- {}", format_path(&path, &rm));
        }
    }

    Ok(())
}

fn format_path(path: &[Recipe], rm: &RecipeMap) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let mut prev_result = u32::MAX;
    for (i, r) in path.iter().enumerate() {
        let pair = [rm.name(r.pair[0]), rm.name(r.pair[1])];
        let result = rm.name(r.result);

        if r.pair[0] == prev_result {
            write!(&mut out, " + {} -> {}", pair[1], result).unwrap();
        } else {
            if i != 0 {
                out.push_str(", ");
            }
            write!(&mut out, "{} + {} -> {}", pair[0], pair[1], result).unwrap();
        }
        prev_result = r.result;
    }
    out
}

fn compare_against_optimal(search: &Search, rm: &RecipeMap) -> Result<(), Box<dyn Error>> {
    let text = std::fs::read_to_string("best_recipes_depth_9.txt")?;
    let mut num_found = 0;
    let mut num_suboptimal = 0;
    let mut num_suboptimal2 = 0;
    for (target_name, recipe_lines) in read_best_recipes(&text) {
        let target = match rm.get_id(target_name) {
            Some(target) => target,
            None => {
                // println!("{}: not found", target_name);
                continue;
            }
        };
        let optimal = recipe_lines.len();
        let card = search.min_card(target);
        assert!(optimal <= card);
        if optimal < card {
            if optimal + 1 < card {
                // println!("Suboptimal {:?}: {} vs {}", target_name, card, optimal);
                num_suboptimal2 += 1;
            }
            num_suboptimal += 1;
        }
        num_found += 1;
    }
    println!(
        "{}/{}/{} paths are suboptimal",
        num_suboptimal2, num_suboptimal, num_found
    );
    Ok(())
}

fn read_best_recipes(text: &str) -> Vec<(&str, Vec<&str>)> {
    let mut res = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if line.is_empty() {
            continue;
        }
        let (_, name) = line.split_once(": ").unwrap();
        let name = name.trim_end_matches(':');
        let recipes = lines.by_ref().take_while(|l| !l.is_empty()).collect();
        res.push((name, recipes));
    }
    res
}

fn name_to_id(name: &str, rm: &RecipeMap) -> Result<u32, String> {
    if let Some(id) = rm.get_id(name) {
        Ok(id)
    } else {
        Err(format!("{:?} not found", name))
    }
}
