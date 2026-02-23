use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::{BuildHasher, Hash};
use rayon::prelude::*;
use strsim::levenshtein;

use crate::config::traits::{Tagging, TaggingItem};

/// Internal helper retained for experimenting with generic `HashMap` bounds.
/// Not used in production logic.
fn _func<'a, K, V, S>(_v: &'a HashMap<K, V, S>)
where
    K: Hash + Eq + Sync,
    V: Sync,
    S: BuildHasher
{
}

/// Selects the best-matching config entry for a set of OCR / source text blocks.
///
/// # Algorithm
/// 1. For every text block in `texts`, compute the minimum Levenshtein distance
///    between each known `tag` and any same-length sliding window in the text.
///    This produces a global `tag → best_distance` map.
/// 2. Each config entry is scored by summing the distances for all of its tags,
///    then normalizing the sum by the character length of the entry's compare string.
/// 3. Entries that have *primary tags* are only kept if every primary tag matches
///    closely enough (normalized distance ≤ 0.34). Entries without primary tags
///    are always kept as fallback candidates.
/// 4. Candidates are sorted: entries with satisfied primary tags come first;
///    within the same primary-tag tier the entry with the lowest normalized score
///    wins.
///
/// # Arguments
/// * `texts`  – One or more text blocks extracted from the input (e.g. OCR lines).
/// * `tags`   – The full set of recognizable tag strings drawn from the config.
/// * `config` – Map from a typed key `T` to a config value `P` that exposes its tags.
///
/// # Returns
/// `Some((best_key, normalized_score))` for the winning entry, or `None` if no
/// entry survived the primary-tag filter.
///
/// # Type parameters
/// * `T` – Config key type; must implement [`Tagging`] and standard collection traits.
/// * `P` – Config value type; must implement [`TaggingItem`] with an iterable tag collection.
pub fn get_config_item<T, P>(texts: &Vec<String>, tags: &HashSet<String>, config: &HashMap<T, P>) -> Option<(T, f64)>
where
    P: TaggingItem + Sync,
    for<'a> &'a P::OutTags: IntoIterator<Item = &'a String>,
    T: Tagging + Hash + Eq + Sync + Send + Clone + Debug,
{
    // Step 1 – build a merged tag→distance map from all text blocks in parallel.
    // Because we collect into a HashMap the last write per tag key wins, which is
    // acceptable: we only need an approximate proximity measure.
    let tags: HashMap<String, usize> = texts.par_iter()
        .map(|text| get_config_tags(text, tags))
        .collect::<Vec<Vec<(String, usize)>>>()
        .concat()
        .into_iter()
        .collect();

    // Step 2 – score every config entry in parallel.
    // Each tuple element: (key, normalized_score, primary_tags_satisfied).
    let mut fishes: Vec<(T, f64, bool)> = config.into_par_iter()
        .filter_map(|(name, fish)| {
            // Accumulate raw distance from all tags that belong to this entry.
            // Tags absent from the distance map contribute 0 (perfect absence score).
            let dist = fish.get_tags().into_iter()
                .fold(0, |acc, tag| {
                    acc + match tags.get(tag) {
                        Some(d) => *d,
                        None => 0,
                    }
                });

            // Evaluate primary tags:
            //   Some(true)  – at least one primary tag matched and none failed.
            //   Some(false) – at least one primary tag failed the threshold.
            //   None        – no primary tags defined for this entry.
            let primary = fish.get_primary_tags();
            let primary_status = primary.into_iter()
                .fold(None, |acc, tag| {
                    match tags.get(tag) {
                        Some(d) => {
                            // Accept the primary tag if its normalized distance is ≤ 0.34.
                            if (*d as f64) / (tag.chars().count() as f64) <= 0.34 {
                                match acc {
                                    Some(e) => Some(e), // keep an existing failure flag
                                    None => Some(true), // first passing tag
                                }
                            } else {
                                Some(false) // tag too dissimilar – mark the entry as failed
                            }
                        },
                        // Tag not found in the distance map → treat as a failure.
                        None => Some(false),
                    }
                });

            // Normalize the total distance by the number of characters in the key's
            // compare string so that entries with many tags are not unfairly penalized.
            let normalized = (dist as f64) / (name.get_compare_str().chars().count() as f64);

            match primary_status {
                // Primary tags were evaluated and all passed → keep with flag=true.
                Some(true)  => Some((name.clone(), normalized, true)),
                // At least one primary tag failed → discard this entry entirely.
                Some(false) => None,
                // No primary tags defined → keep as a fallback candidate with flag=false.
                None        => Some((name.clone(), normalized, false)),
            }
        }).collect();

    // Step 3 – sort: primary-tag-satisfied entries first, then by ascending score.
    // Lower normalized score means a closer overall match.
    fishes.sort_unstable_by(|left, right| {
        if left.2 == right.2 {
            // Same primary-tag tier → prefer the lower score.
            return left.1.partial_cmp(&right.1).unwrap();
        }
        // Different tier → entries with primary tags satisfied (true > false) rank first.
        right.2.partial_cmp(&left.2).unwrap()
    });

    // No candidates survived the filters.
    if fishes.is_empty() {
        return None;
    }

    // Return the top-ranked candidate (key + its normalized score).
    Some(fishes.into_iter().map(|obj| (obj.0, obj.1)).collect::<Vec<(T, f64)>>()[0].clone())
}

/// Computes the minimum Levenshtein distance between each tag and the given text.
///
/// For every tag in `tags` the function slides a window of the same character length
/// across `text` and records the smallest edit distance found. Newlines in the text
/// are replaced with spaces before windowing so multi-line inputs are handled uniformly.
///
/// The computation is parallelized with Rayon across the tag set.
///
/// # Arguments
/// * `text` – A single text block to search within (may contain newlines).
/// * `tags` – Set of tag strings to look for.
///
/// # Returns
/// A vector of `(tag, min_distance)` pairs — one entry per tag.
/// If the text is shorter than a tag, the window iterator yields no values and the
/// distance remains [`usize::MAX`], signalling that the tag is effectively absent.
fn get_config_tags(text: &String, tags: &HashSet<String>) -> Vec<(String, usize)> {
    tags.par_iter()
        .filter_map(|tag| {
            // Normalize newlines so the sliding window sees a flat character sequence.
            let flat = text.replace('\n', " ");
            let chars: Vec<char> = flat.chars().collect();

            // Slide a window of exactly `tag.len()` characters and keep the minimum distance.
            let min_dist = chars
                .windows(tag.chars().count())
                .fold(usize::MAX, |acc, window| {
                    let dist = levenshtein(tag, &window.iter().collect::<String>());
                    if dist < acc { dist } else { acc }
                });

            Some((tag.to_string(), min_dist))
        })
        .collect()
}