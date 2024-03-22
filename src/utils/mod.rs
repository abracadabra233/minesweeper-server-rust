use serde::{Deserialize, Serialize};
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

pub fn show_matrix<T: std::fmt::Display>(matrix: &[Vec<T>], name: &str) {
    println!("============= {} =============", name);
    for row in matrix {
        for item in row.iter() {
            print!("{}\t", item);
        }
        println!();
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}
