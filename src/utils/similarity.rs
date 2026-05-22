use strsim::normalized_levenshtein;

pub fn calculate_similarity(a: &str, b: &str) -> f64 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    normalized_levenshtein(&a_lower, &b_lower)
}
