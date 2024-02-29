use rand::{distributions::Alphanumeric, Rng};
pub fn generate_room_id() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect()
}
