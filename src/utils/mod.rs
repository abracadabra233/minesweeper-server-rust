pub mod neighbors;
use rand::{distributions::Alphanumeric, Rng};
pub fn generate_room_id() -> String {
    return "66666".to_string();
    // rand::thread_rng()
    //     .sample_iter(&Alphanumeric)
    //     .take(6)
    //     .map(char::from)
    //     .collect()
}
