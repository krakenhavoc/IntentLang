//! Fuzzy matching utilities for "did you mean?" suggestions.
//!
//! Uses Levenshtein distance to find similar names when type or field
//! references fail to resolve, providing actionable suggestions to the user.

/// Compute the Levenshtein (edit) distance between two strings.
///
/// The edit distance is the minimum number of single-character insertions,
/// deletions, or substitutions required to transform one string into the other.
pub fn levenshtein(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut dp: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut prev = dp[0];
        dp[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            let temp = dp[j];
            dp[j] = (dp[j] + 1).min(dp[j - 1] + 1).min(prev + cost);
            prev = temp;
        }
    }
    dp[b.len()]
}

/// Find the most similar name from `candidates` within `max_distance` edits.
///
/// Returns the best match if one exists within the threshold.
/// If multiple candidates tie at the same distance, the first one found is returned.
pub fn find_similar(name: &str, candidates: &[&str], max_distance: usize) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    for &candidate in candidates {
        let dist = levenshtein(name, candidate);
        if dist > 0 && dist <= max_distance && (best.is_none() || dist < best.unwrap().0) {
            best = Some((dist, candidate));
        }
    }
    best.map(|(_, s)| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_identical() {
        assert_eq!(levenshtein("hello", "hello"), 0);
    }

    #[test]
    fn levenshtein_empty() {
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", ""), 3);
        assert_eq!(levenshtein("", ""), 0);
    }

    #[test]
    fn levenshtein_single_edit() {
        // substitution
        assert_eq!(levenshtein("cat", "car"), 1);
        // insertion
        assert_eq!(levenshtein("cat", "cats"), 1);
        // deletion
        assert_eq!(levenshtein("cats", "cat"), 1);
    }

    #[test]
    fn levenshtein_multiple_edits() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("Customer", "Cusotmer"), 2);
    }

    #[test]
    fn find_similar_exact_match_excluded() {
        // Exact match (distance 0) should not be returned.
        let result = find_similar("Account", &["Account", "Order"], 2);
        assert!(result.is_none());
    }

    #[test]
    fn find_similar_typo() {
        let result = find_similar("Cusotmer", &["Customer", "Account", "Order"], 2);
        assert_eq!(result, Some("Customer".to_string()));
    }

    #[test]
    fn find_similar_no_close_match() {
        let result = find_similar("XyzAbc", &["Customer", "Account", "Order"], 2);
        assert!(result.is_none());
    }

    #[test]
    fn find_similar_field_typo() {
        let result = find_similar("balence", &["balance", "id", "status"], 2);
        assert_eq!(result, Some("balance".to_string()));
    }

    #[test]
    fn find_similar_picks_closest() {
        // "stat" is distance 2 from "status" and distance 3 from "state_machine"
        let result = find_similar("stat", &["status", "state_machine"], 2);
        assert_eq!(result, Some("status".to_string()));
    }
}
