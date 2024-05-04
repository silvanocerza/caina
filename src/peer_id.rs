use rand::{distributions::Alphanumeric, Rng};

pub fn generate_peer_id() -> String {
    let random_id_suffix: String = rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(12)
        .map(|b| format!("%{:02X}", b))
        .collect();
    // Use CN000 as prefix, CN from Caina, the name I'll be using for the project
    // 000 is the version, just zeroes for the time being.
    let peer_id_fixed: String = "-CN0000-".bytes().map(|b| format!("%{:02X}", b)).collect();
    format!("{}{}", peer_id_fixed, random_id_suffix)
}
