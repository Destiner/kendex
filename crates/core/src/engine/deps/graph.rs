use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Every set of skills that require each other, each reported once. A cycle
/// is information, not a fault: two items that need one another are a
/// co-install their authors meant.
pub(super) fn cycles(edges: &BTreeMap<String, BTreeSet<String>>) -> Vec<Vec<String>> {
    let mut found: Vec<Vec<String>> = Vec::new();
    for start in edges.keys() {
        let forward = reachable(edges, start);
        if !forward.contains(start) {
            continue;
        }
        // Everything that reaches back is in the same knot as the start.
        let members: Vec<String> = forward
            .into_iter()
            .filter(|name| reachable(edges, name).contains(start))
            .collect();
        if !found.contains(&members) {
            found.push(members);
        }
    }
    found
}

/// Every skill reachable from this one in one or more steps.
fn reachable(edges: &BTreeMap<String, BTreeSet<String>>, start: &String) -> BTreeSet<String> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut queue: VecDeque<&String> = VecDeque::from([start]);
    while let Some(name) = queue.pop_front() {
        for next in edges.get(name).into_iter().flatten() {
            if seen.insert(next.clone()) {
                queue.push_back(next);
            }
        }
    }
    seen
}
