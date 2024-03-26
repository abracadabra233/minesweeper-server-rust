use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
pub mod neighbors;
// use rand::{distributions::Alphanumeric, Rng};
pub fn generate_room_id() -> String {
    "66666".to_string()
    // rand::thread_rng()
    //     .sample_iter(&Alphanumeric)
    //     .take(6)
    //     .map(char::from)
    //     .collect()
}

pub fn show_matrix<T: Display + Eq + std::hash::Hash, V: Display>(
    matrix: &[Vec<T>],
    name: &str,
    replacements: &HashMap<T, V>,
) {
    println!("============= {} =============", name);
    for row in matrix {
        for item in row.iter() {
            match replacements.get(item) {
                Some(replacement) => print!("{} ", replacement),
                None => print!("{}\t", item),
            }
        }
        println!();
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}
