use std::{borrow::Cow, collections::HashMap};

use infinite_craft::RecipeMap;
use rusqlite::{Connection, OpenFlags};

pub fn main() -> anyhow::Result<()> {
    let mut rm = RecipeMap::new();

    let path = "infinite-craft.db";
    if let Ok(conn) = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        println!("Reading {}", path);
        read_pair_all(&conn, &mut rm)?;
    }

    let path = "relevant_recipes.json";
    if let Ok(text) = std::fs::read_to_string(path) {
        println!("Reading {}", path);
        read_relevant_recipes(&text, &mut rm)?;
    }

    let path = "helper-recipes.db";
    if let Ok(conn) = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        println!("Reading {}", path);
        read_helper_recipes(&conn, &mut rm)?;
    }

    println!("{} items, {} recipes", rm.num_items(), rm.num_recipes());

    let path = "recipe-map.bincode";
    std::fs::write(path, &bincode::serialize(&rm.as_serializable())?)?;
    println!("Written {}", path);

    Ok(())
}

pub fn read_pair_all(conn: &Connection, rm: &mut RecipeMap) -> anyhow::Result<()> {
    let mut stmt = conn.prepare("select first, second, result from pair")?;
    let mut iter = stmt.query(())?;
    while let Some(row) = iter.next()? {
        let first = row.get_ref(0)?.as_str()?;
        let second = row.get_ref(1)?.as_str()?;
        if let Some(result) = row.get_ref(2)?.as_str_or_null()? {
            rm.insert(first, second, result);
        }
    }
    Ok(())
}

pub fn read_relevant_recipes(text: &str, rm: &mut RecipeMap) -> anyhow::Result<()> {
    let data: HashMap<Cow<'_, str>, Vec<[Cow<'_, str>; 2]>> = serde_json::from_str(text)?;
    let mut data = data.into_iter().collect::<Vec<_>>();
    data.sort_by(|x, y| x.0.cmp(&y.0));
    for (result, pairs) in data {
        for [first, second] in pairs {
            if first.is_ascii() && second.is_ascii() && result.is_ascii() {
                rm.insert(first.as_ref(), second.as_ref(), result.as_ref());
            }
        }
    }
    Ok(())
}

pub fn read_helper_recipes(conn: &Connection, rm: &mut RecipeMap) -> anyhow::Result<()> {
    let mut stmt = conn.prepare("select first, second, result from recipes")?;
    let mut iter = stmt.query(())?;
    while let Some(row) = iter.next()? {
        let first = row.get_ref(0)?.as_str()?;
        let second = row.get_ref(1)?.as_str()?;
        let result = row.get_ref(2)?.as_str()?;
        if result == "Nothing" {
            continue;
        }
        rm.insert(first, second, result);
    }
    Ok(())
}
