use crate::model::{AppData, GenerateRequest, PracticeSection, SessionPlan};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

const PAIRS: &[&str] = &[
    "fj", "dk", "sl", "a;", "gh", "ru", "ei", "wo", "qy", "tp", "vm", "c,", "x.", "z/", "bn",
];
const SECTION_NAMES: [&str; 6] = [
    "Warm-up",
    "Basic patterns",
    "Alternation",
    "Transition practice",
    "Difficult combinations",
    "Longer sequences",
];

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn range(&mut self, start: usize, end: usize) -> usize {
        start + self.next() as usize % (end - start)
    }
    fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            let j = self.range(0, i + 1);
            values.swap(i, j);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Template {
    Balanced,
    Alternation,
    Transitions,
    Precision,
    Endurance,
    WeakFocus,
}
impl Template {
    fn choose(seed: u64, has_weakness: bool) -> Self {
        match seed % if has_weakness { 6 } else { 5 } {
            0 => Self::Balanced,
            1 => Self::Alternation,
            2 => Self::Transitions,
            3 => Self::Precision,
            4 => Self::Endurance,
            _ => Self::WeakFocus,
        }
    }
    fn allocation(self) -> [usize; 6] {
        match self {
            Self::Balanced => [4, 4, 4, 4, 4, 4],
            Self::Alternation => [4, 6, 5, 4, 3, 2],
            Self::Transitions => [3, 3, 4, 7, 4, 3],
            Self::Precision => [6, 5, 4, 3, 3, 3],
            Self::Endurance => [3, 3, 3, 4, 5, 6],
            Self::WeakFocus => [3, 3, 3, 4, 6, 5],
        }
    }
}

#[derive(Default)]
struct AdaptiveWeights {
    keys: BTreeMap<char, u64>,
    transitions: BTreeMap<String, u64>,
}
impl AdaptiveWeights {
    fn has_weakness(&self) -> bool {
        self.keys
            .values()
            .chain(self.transitions.values())
            .any(|&w| w > 1)
    }
}

pub fn generate(req: &GenerateRequest, data: &AppData) -> Result<SessionPlan, String> {
    let pairs = validate(req)?;
    let seed = req.seed.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    });
    let total = match req.length.as_str() {
        "short" => 24,
        "medium" | "endless" => 48,
        "long" => 90,
        _ => return Err("Unknown session length".into()),
    };
    let adaptive = adaptive_weights(&pairs, data, req.adaptive);
    let template = Template::choose(seed, adaptive.has_weakness());
    let counts = section_counts(total, template.allocation());
    let extended = matches!(req.length.as_str(), "long" | "endless");
    let pair_chars: Vec<[char; 2]> = pairs
        .iter()
        .map(|p| {
            let mut chars = p.chars();
            [chars.next().unwrap(), chars.next().unwrap()]
        })
        .collect();
    let mut rng = Rng::new(seed);
    let coverage = transition_schedule(&pair_chars, &mut rng);
    let mut used = BTreeSet::new();
    let mut sections = Vec::with_capacity(6);

    for (stage, name) in SECTION_NAMES.iter().enumerate() {
        let mut lines = Vec::with_capacity(counts[stage]);
        for line_index in 0..counts[stage] {
            let mut line = String::new();
            for attempt in 0..12 {
                line = if req.mode == "standalone" {
                    standalone(
                        pair_chars[0],
                        stage,
                        line_index + attempt,
                        template,
                        extended,
                        &adaptive,
                        &mut rng,
                    )
                } else if stage == 3 && pair_chars.len() > 1 {
                    coverage_line(&coverage, counts[stage], line_index, attempt)
                } else {
                    mixed(
                        &pair_chars,
                        stage,
                        line_index + attempt,
                        template,
                        extended,
                        &adaptive,
                        &mut rng,
                    )
                };
                if used.insert(line.clone()) {
                    break;
                }
            }
            lines.push(line);
        }
        sections.push(PracticeSection {
            name: (*name).into(),
            lines,
        });
    }
    Ok(SessionPlan {
        mode: req.mode.clone(),
        pairs,
        sections,
        seed,
    })
}

fn validate(req: &GenerateRequest) -> Result<Vec<String>, String> {
    if req.pairs.is_empty() {
        return Err("Select at least one key pair".into());
    }
    let mut seen = BTreeSet::new();
    let mut pairs = Vec::new();
    for raw in &req.pairs {
        let pair = raw.to_lowercase();
        if !PAIRS.contains(&pair.as_str()) {
            return Err(format!("Unknown key pair: {raw}"));
        }
        if seen.insert(pair.clone()) {
            pairs.push(pair);
        }
    }
    if req.mode == "standalone" && pairs.len() != 1 {
        return Err("Standalone practice requires exactly one pair".into());
    }
    if req.mode != "standalone" && req.mode != "mixed" {
        return Err("Unknown practice mode".into());
    }
    Ok(pairs)
}

fn section_counts(total: usize, weights: [usize; 6]) -> [usize; 6] {
    let weight_total: usize = weights.iter().sum();
    let mut counts = weights.map(|weight| (total * weight / weight_total).max(1));
    while counts.iter().sum::<usize>() < total {
        let index = counts
            .iter()
            .enumerate()
            .min_by_key(|(i, count)| (**count * weight_total, usize::MAX - weights[*i]))
            .unwrap()
            .0;
        counts[index] += 1;
    }
    while counts.iter().sum::<usize>() > total {
        let index = counts
            .iter()
            .enumerate()
            .filter(|(_, count)| **count > 1)
            .max_by_key(|(i, count)| **count * weight_total / weights[*i])
            .unwrap()
            .0;
        counts[index] -= 1;
    }
    counts
}

const STANDALONE: [&[&str]; 6] = [
    &["0000", "1111", "000", "111", "00", "11", "00000", "11111"],
    &["01", "10", "0101", "1010", "01010", "10101"],
    &["001", "110", "011", "100", "0010", "1101", "0110", "1001"],
    &[
        "0100", "1011", "0011", "1100", "0110", "1001", "01001", "10110",
    ],
    &[
        "01010", "10101", "00101", "11010", "01101", "10010", "010110", "101001",
    ],
    &[
        "01010011",
        "10101100",
        "00101101",
        "11010010",
        "01011001",
        "10100110",
        "00110101",
        "11001010",
        "0100110101",
        "1011001010",
    ],
];

fn standalone(
    pair: [char; 2],
    stage: usize,
    line_index: usize,
    template: Template,
    extended: bool,
    adaptive: &AdaptiveWeights,
    rng: &mut Rng,
) -> String {
    let patterns = STANDALONE[stage];
    let group_count = if stage < 2 { 6 } else { 5 };
    let mut order: Vec<usize> = (0..patterns.len()).collect();
    rng.shuffle(&mut order);
    order.rotate_left(line_index % patterns.len());
    let adaptive_groups = if stage >= 4 && adaptive.has_weakness() {
        if template == Template::WeakFocus {
            2
        } else {
            1
        }
    } else {
        0
    };
    (0..group_count)
        .map(|group| {
            if group >= group_count - adaptive_groups {
                let len = if extended { 6 + stage } else { 2 * stage };
                adaptive_walk(&pair, len, adaptive, rng)
            } else {
                render_pattern(patterns[order[group % order.len()]], pair)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_pattern(pattern: &str, pair: [char; 2]) -> String {
    pattern
        .bytes()
        .map(|index| pair[(index - b'0') as usize])
        .collect()
}

fn mixed(
    pairs: &[[char; 2]],
    stage: usize,
    line_index: usize,
    template: Template,
    extended: bool,
    adaptive: &AdaptiveWeights,
    rng: &mut Rng,
) -> String {
    if pairs.len() == 1 {
        return standalone(
            pairs[0], stage, line_index, template, extended, adaptive, rng,
        );
    }
    let keys: Vec<char> = pairs.iter().flatten().copied().collect();
    let groups = if stage < 2 { 6 } else { 5 };
    let adaptive_groups = if stage >= 4 && adaptive.has_weakness() {
        if template == Template::WeakFocus {
            2
        } else {
            1
        }
    } else {
        0
    };
    let mut indices: Vec<usize> = (0..groups)
        .map(|group| line_index * groups + group)
        .collect();
    rng.shuffle(&mut indices);
    (0..groups)
        .map(|group| {
            if group >= groups - adaptive_groups {
                let len = if extended { 7 + stage } else { 2 * stage };
                return adaptive_walk(&keys, len, adaptive, rng);
            }
            let index = indices[group];
            match stage {
                0 => repeated_key(&keys, index),
                1 => pair_alternation(pairs, index),
                2 => short_cross_pair(pairs, index),
                4 => difficult_cross_pair(pairs, index),
                5 => longer_balanced(pairs, index, if extended { 9 } else { 7 } + index % 4),
                _ => unreachable!(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn repeated_key(keys: &[char], index: usize) -> String {
    std::iter::repeat_n(keys[index % keys.len()], 2 + index % 3).collect()
}

fn pair_alternation(pairs: &[[char; 2]], index: usize) -> String {
    let pair = pairs[index % pairs.len()];
    let patterns = ["01", "10", "0101", "1010", "0010", "1101"];
    render_pattern(patterns[(index / pairs.len()) % patterns.len()], pair)
}

fn short_cross_pair(pairs: &[[char; 2]], index: usize) -> String {
    let source = index % pairs.len();
    let round = index / pairs.len();
    let target = (source + 1 + round % (pairs.len() - 1)) % pairs.len();
    let a = pairs[source];
    let b = pairs[target];
    match round % 6 {
        0 => [a[0], b[0], a[1]].into_iter().collect(),
        1 => [a[1], b[1], a[0]].into_iter().collect(),
        2 => [a[0], b[1], a[1], b[0]].into_iter().collect(),
        3 => [b[1], a[1], b[0], a[0]].into_iter().collect(),
        4 => [a[0], a[1], b[0], b[1], a[0]].into_iter().collect(),
        _ => [b[1], b[0], a[1], a[0], b[1]].into_iter().collect(),
    }
}

fn difficult_cross_pair(pairs: &[[char; 2]], index: usize) -> String {
    let source = index % pairs.len();
    let round = index / pairs.len();
    let target = (source + 1 + round % (pairs.len() - 1)) % pairs.len();
    let a = pairs[source];
    let b = pairs[target];
    match round % 4 {
        0 => [a[0], b[0], b[0], a[1], b[1], a[0]].into_iter().collect(),
        1 => [b[1], a[1], a[1], b[0], a[0], b[1]].into_iter().collect(),
        2 => [a[0], b[1], a[1], b[0], b[1], a[0], a[1]]
            .into_iter()
            .collect(),
        _ => [b[0], a[1], b[1], a[0], a[1], b[0], b[1]]
            .into_iter()
            .collect(),
    }
}

fn longer_balanced(pairs: &[[char; 2]], index: usize, len: usize) -> String {
    let mut order: Vec<char> = match index % 4 {
        0 => pairs.iter().flatten().copied().collect(),
        1 => pairs
            .iter()
            .rev()
            .flat_map(|pair| pair.iter().rev())
            .copied()
            .collect(),
        2 => pairs
            .iter()
            .map(|pair| pair[0])
            .chain(pairs.iter().map(|pair| pair[1]))
            .collect(),
        _ => pairs
            .iter()
            .rev()
            .map(|pair| pair[1])
            .chain(pairs.iter().rev().map(|pair| pair[0]))
            .collect(),
    };
    let rotation = index / 4 % order.len();
    order.rotate_left(rotation);
    (0..len)
        .map(|i| {
            if i > 1 && i % 6 == 4 {
                order[(i - 1) % order.len()]
            } else {
                order[i % order.len()]
            }
        })
        .collect()
}

fn transition_schedule(pairs: &[[char; 2]], rng: &mut Rng) -> Vec<[char; 2]> {
    if pairs.len() < 2 {
        return vec![];
    }
    let mut order: Vec<usize> = (0..pairs.len()).collect();
    rng.shuffle(&mut order);
    let mut edges = Vec::with_capacity(4 * pairs.len() * (pairs.len() - 1));
    for offset in 1..pairs.len() {
        let side_order = if rng.next() & 1 == 0 { [0, 1] } else { [1, 0] };
        for source in 0..pairs.len() {
            let a = pairs[order[source]];
            let b = pairs[order[(source + offset) % pairs.len()]];
            for &left in &side_order {
                for &right in &side_order {
                    edges.push([a[left], b[right]]);
                }
            }
        }
    }
    edges
}

fn coverage_line(edges: &[[char; 2]], line_count: usize, line: usize, attempt: usize) -> String {
    let pair_count = ((1.0 + (1.0 + edges.len() as f64).sqrt()) / 2.0).round() as usize;
    let block = (pair_count * 4).max(1);
    let capacity = (line_count * 12).max(block);
    let covered = if edges.len() <= capacity {
        edges.len()
    } else {
        capacity / block * block
    };
    let target = covered.max(line_count * 6).div_ceil(block) * block;
    let base = target / line_count;
    let extra = target % line_count;
    let line_len = base + usize::from(line < extra);
    let start = line * base + line.min(extra) + attempt;
    (0..line_len)
        .map(|i| edges[(start + i) % covered])
        .map(|edge| edge.into_iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

fn adaptive_walk(keys: &[char], len: usize, adaptive: &AdaptiveWeights, rng: &mut Rng) -> String {
    let mut out = String::with_capacity(len);
    out.push(weighted_key(keys, None, adaptive, rng));
    while out.len() < len {
        let previous = out.chars().last();
        out.push(weighted_key(keys, previous, adaptive, rng));
    }
    out
}

fn weighted_key(
    keys: &[char],
    previous: Option<char>,
    adaptive: &AdaptiveWeights,
    rng: &mut Rng,
) -> char {
    let weights: Vec<u64> = keys
        .iter()
        .map(|key| {
            let key_weight = adaptive.keys.get(key).copied().unwrap_or(1);
            let transition_weight = previous
                .and_then(|from| adaptive.transitions.get(&format!("{from}{key}")))
                .copied()
                .unwrap_or(1);
            key_weight.max(transition_weight).min(3)
        })
        .collect();
    let total: u64 = weights.iter().sum();
    let mut choice = rng.next() % total;
    for (&key, weight) in keys.iter().zip(weights) {
        if choice < weight {
            return key;
        }
        choice -= weight;
    }
    keys[0]
}

fn adaptive_weights(pairs: &[String], data: &AppData, enabled: bool) -> AdaptiveWeights {
    if !enabled {
        return AdaptiveWeights::default();
    }
    let allowed: BTreeSet<char> = pairs.iter().flat_map(|pair| pair.chars()).collect();
    let mut transitions = BTreeMap::new();
    for &from in &allowed {
        for &to in &allowed {
            let transition = format!("{from}{to}");
            let errors = data
                .transition_errors
                .get(&transition)
                .copied()
                .unwrap_or(0);
            let attempts = data
                .transition_counts
                .get(&transition)
                .copied()
                .unwrap_or(errors);
            let slow = data
                .transition_timings
                .get(&transition)
                .and_then(|timing| timing.total_ms.checked_div(timing.samples))
                .unwrap_or(0)
                > 450;
            if errors > 0 || slow {
                let rate = errors as f64 / attempts.max(1) as f64;
                transitions.insert(transition, weakness_weight(rate, slow));
            }
        }
    }
    let keys = allowed
        .iter()
        .filter_map(|&key| {
            let name = key.to_string();
            let errors = data.key_errors.get(&name).copied().unwrap_or(0);
            let slow = data
                .key_timings
                .get(&name)
                .and_then(|timing| timing.total_ms.checked_div(timing.samples))
                .unwrap_or(0)
                > 450;
            (errors > 0 || slow).then(|| {
                let attempts: u64 = data
                    .transition_counts
                    .iter()
                    .filter(|(transition, _)| transition.chars().nth(1) == Some(key))
                    .map(|(_, count)| count)
                    .sum::<u64>()
                    .max(errors)
                    .max(1);
                let rate = errors as f64 / attempts as f64;
                (key, weakness_weight(rate, slow))
            })
        })
        .collect();
    AdaptiveWeights { keys, transitions }
}

fn weakness_weight(error_rate: f64, slow: bool) -> u64 {
    1 + (error_rate > 0.08) as u64 + (error_rate > 0.20 || slow) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(pairs: &[&str], mode: &str, seed: u64) -> GenerateRequest {
        GenerateRequest {
            pairs: pairs.iter().map(|pair| pair.to_string()).collect(),
            mode: mode.into(),
            length: "short".into(),
            seed: Some(seed),
            adaptive: false,
        }
    }
    fn lines(plan: &SessionPlan) -> impl Iterator<Item = &String> {
        plan.sections.iter().flat_map(|section| &section.lines)
    }
    fn text(plan: &SessionPlan) -> String {
        lines(plan).cloned().collect::<Vec<_>>().join(" ")
    }
    fn key_counts(plan: &SessionPlan, keys: &str) -> BTreeMap<char, usize> {
        let all = text(plan);
        keys.chars()
            .map(|key| (key, all.matches(key).count()))
            .collect()
    }
    fn transitions(plan: &SessionPlan) -> BTreeSet<String> {
        lines(plan)
            .flat_map(|line| line.split_whitespace())
            .flat_map(|group| {
                let chars: Vec<char> = group.chars().collect();
                chars
                    .windows(2)
                    .map(|edge| edge.iter().collect())
                    .collect::<Vec<String>>()
            })
            .collect()
    }
    fn average_group_length(section: &PracticeSection) -> f64 {
        let groups: Vec<&str> = section
            .lines
            .iter()
            .flat_map(|line| line.split_whitespace())
            .collect();
        groups.iter().map(|group| group.len()).sum::<usize>() as f64 / groups.len() as f64
    }

    fn group_lengths(plan: &SessionPlan) -> Vec<usize> {
        lines(plan)
            .flat_map(|line| line.split_whitespace().map(str::len))
            .collect()
    }

    #[test]
    fn empty_and_invalid_selections_are_safe() {
        assert!(generate(&req(&[], "mixed", 1), &AppData::default()).is_err());
        assert!(generate(&req(&["fj", "??"], "mixed", 1), &AppData::default()).is_err());
        assert!(generate(&req(&["fj", "dk"], "standalone", 1), &AppData::default()).is_err());
    }

    #[test]
    fn standalone_has_deliberate_progression_for_every_pair() {
        for &pair in PAIRS {
            let plan = generate(&req(&[pair], "standalone", 17), &AppData::default()).unwrap();
            let allowed = format!("{} ", pair);
            assert!(
                text(&plan).chars().all(|key| allowed.contains(key)),
                "unexpected key for {pair}"
            );
            let chars: Vec<char> = pair.chars().collect();
            let warm = plan.sections[0].lines.join(" ");
            assert!(warm.contains(&chars[0].to_string().repeat(3)));
            assert!(warm.contains(&chars[1].to_string().repeat(3)));
            let basic = plan.sections[1].lines.join(" ");
            assert!(basic.contains(&format!("{}{}", chars[0], chars[1])));
            assert!(basic.contains(&format!("{}{}", chars[1], chars[0])));
            assert!(
                average_group_length(&plan.sections[5])
                    > average_group_length(&plan.sections[0]) + 4.0
            );
        }
    }

    #[test]
    fn mixed_uses_only_selected_keys_and_balances_them() {
        for seed in 1..=20 {
            let mut request = req(&["fj", "dk", "sl", "a;"], "mixed", seed);
            request.length = "long".into();
            let plan = generate(&request, &AppData::default()).unwrap();
            assert!(text(&plan).chars().all(|key| "fjdksla; ".contains(key)));
            let counts = key_counts(&plan, "fjdksla;");
            let min = *counts.values().min().unwrap() as f64;
            let max = *counts.values().max().unwrap() as f64;
            assert!(max / min < 1.30, "seed {seed} is imbalanced: {counts:?}");
        }
    }

    #[test]
    fn mixed_covers_every_direction_between_two_pairs() {
        let plan = generate(&req(&["fj", "dk"], "mixed", 31), &AppData::default()).unwrap();
        let found = transitions(&plan);
        for edge in ["fd", "fk", "jd", "jk", "df", "dj", "kf", "kj"] {
            assert!(found.contains(edge), "missing {edge}");
        }
    }

    #[test]
    fn four_pairs_cover_the_complete_directional_transition_space() {
        let plan = generate(
            &req(&["fj", "dk", "sl", "a;"], "mixed", 44),
            &AppData::default(),
        )
        .unwrap();
        let found = transitions(&plan);
        let keys: Vec<char> = "fjdksla;".chars().collect();
        for &from in &keys {
            for &to in &keys {
                if from != to {
                    assert!(found.contains(&format!("{from}{to}")), "missing {from}{to}");
                }
            }
        }
    }

    #[test]
    fn mixed_is_not_concatenated_standalone_practice() {
        let mixed = generate(&req(&["fj", "dk"], "mixed", 51), &AppData::default()).unwrap();
        let cross = transitions(&mixed)
            .iter()
            .filter(|edge| {
                let chars: Vec<char> = edge.chars().collect();
                "fj".contains(chars[0]) && "dk".contains(chars[1])
                    || "dk".contains(chars[0]) && "fj".contains(chars[1])
            })
            .count();
        assert!(cross >= 8);
        assert!(mixed.sections[2].lines.iter().all(|line| {
            line.split_whitespace().all(|group| {
                group.chars().any(|key| "fj".contains(key))
                    && group.chars().any(|key| "dk".contains(key))
            })
        }));
    }

    #[test]
    fn punctuation_pairs_are_preserved() {
        let plan = generate(&req(&["c,", "x.", "z/"], "mixed", 63), &AppData::default()).unwrap();
        let generated = text(&plan);
        assert!(generated.contains(',') && generated.contains('.') && generated.contains('/'));
        assert!(generated.chars().all(|key| "c,x.z/ ".contains(key)));
    }

    #[test]
    fn sections_become_longer_and_more_varied() {
        let plan = generate(&req(&["fj", "dk"], "mixed", 72), &AppData::default()).unwrap();
        assert!(
            average_group_length(&plan.sections[5]) > average_group_length(&plan.sections[0]) + 5.0
        );
        assert!(transitions(&plan).len() >= 12);
    }

    #[test]
    fn short_and_medium_sessions_keep_dense_chunks_in_the_minority() {
        let mut data = AppData::default();
        data.transition_errors.insert("fd".into(), 20);
        data.transition_counts.insert("fd".into(), 25);
        for length in ["short", "medium"] {
            let mut request = req(&["fj", "dk", "sl", "a;"], "mixed", 4);
            request.length = length.into();
            request.adaptive = true;
            let plan = generate(&request, &data).unwrap();
            let lengths = group_lengths(&plan);
            assert!(lengths.iter().all(|&len| len <= 10));
            assert!(lengths.iter().filter(|&&len| len <= 6).count() * 2 > lengths.len());
            assert!(lengths.iter().filter(|&&len| len >= 10).count() * 4 < lengths.len());
        }

        let mut long = req(&["fj", "dk"], "mixed", 4);
        long.length = "long".into();
        assert!(group_lengths(&generate(&long, &data).unwrap())
            .iter()
            .any(|&len| len > 10));
    }

    #[test]
    fn exact_lines_are_not_repeated_excessively() {
        for seed in 80..100 {
            let mut request = req(&["fj"], "standalone", seed);
            request.length = "long".into();
            let plan = generate(&request, &AppData::default()).unwrap();
            let all: Vec<&String> = lines(&plan).collect();
            let unique: BTreeSet<&String> = all.iter().copied().collect();
            assert!(
                unique.len() * 100 / all.len() >= 95,
                "seed {seed} repeated too many lines"
            );
        }
    }

    #[test]
    fn session_templates_change_the_emphasis() {
        let signatures: BTreeSet<Vec<usize>> = (1..=12)
            .map(|seed| {
                generate(&req(&["fj", "dk"], "mixed", seed), &AppData::default())
                    .unwrap()
                    .sections
                    .iter()
                    .map(|section| section.lines.len())
                    .collect()
            })
            .collect();
        assert!(signatures.len() >= 5);
    }

    #[test]
    fn adaptive_weighting_reinforces_without_overwhelming() {
        let mut data = AppData::default();
        data.transition_errors.insert("fd".into(), 40);
        data.transition_counts.insert("fd".into(), 40);
        data.key_errors.insert("d".into(), 30);
        for edge in ["jd", "kd", "dd"] {
            data.transition_counts.insert(edge.into(), 40);
        }
        let mut plain_total = 0;
        let mut adaptive_total = 0;
        let mut plain_key_total = 0;
        let mut adaptive_key_total = 0;
        for seed in 100..140 {
            let mut plain = req(&["fj", "dk"], "mixed", seed);
            plain.length = "long".into();
            let plain_plan = generate(&plain, &data).unwrap();
            let plain_text = text(&plain_plan);
            plain_total += plain_text.matches("fd").count();
            plain_key_total += plain_text.matches('d').count();
            let mut weighted = plain;
            weighted.adaptive = true;
            let weighted_plan = generate(&weighted, &data).unwrap();
            let weighted_text = text(&weighted_plan);
            adaptive_total += weighted_text.matches("fd").count();
            adaptive_key_total += weighted_text.matches('d').count();
            assert!(transitions(&weighted_plan).is_superset(&transitions(
                &generate(&req(&["fj", "dk"], "mixed", seed), &AppData::default()).unwrap()
            )));
            assert!(weighted_text.matches("fd").count() * 5 < weighted_text.len());
        }
        assert!(
            adaptive_total as f64 > plain_total as f64 * 1.10,
            "{adaptive_total} not sufficiently above {plain_total}"
        );
        assert!(
            adaptive_key_total as f64 > plain_key_total as f64 * 1.03,
            "problem key exposure did not increase: {adaptive_key_total} vs {plain_key_total}"
        );

        data.transition_counts.insert("fd".into(), 4_000);
        data.key_errors.insert("d".into(), 1);
        let weights = adaptive_weights(&["fj".into(), "dk".into()], &data, true);
        assert_eq!(weights.transitions["fd"], 1);
        assert_eq!(weights.keys[&'d'], 1);
    }

    #[test]
    fn slow_keys_and_transitions_are_reinforced_without_errors() {
        let mut data = AppData::default();
        data.key_timings.insert(
            "j".into(),
            crate::model::TimingStat {
                total_ms: 5_000,
                samples: 10,
            },
        );
        data.transition_timings.insert(
            "jk".into(),
            crate::model::TimingStat {
                total_ms: 5_000,
                samples: 10,
            },
        );
        let weights = adaptive_weights(&["fj".into(), "dk".into()], &data, true);
        assert_eq!(weights.keys[&'j'], 2);
        assert_eq!(weights.transitions["jk"], 2);
    }

    #[test]
    #[ignore = "manual generator inspection; run with: cargo test inspect_samples -- --ignored --nocapture"]
    fn inspect_samples() {
        let mut weak = AppData::default();
        weak.transition_errors.insert("fd".into(), 20);
        weak.transition_counts.insert("fd".into(), 25);
        for (pairs, mode, adaptive, data) in [
            (vec!["fj"], "standalone", false, &AppData::default()),
            (vec!["dk"], "standalone", false, &AppData::default()),
            (vec!["fj", "dk"], "mixed", false, &AppData::default()),
            (vec!["fj", "dk"], "mixed", true, &weak),
            (
                vec!["fj", "dk", "sl", "a;"],
                "mixed",
                false,
                &AppData::default(),
            ),
        ] {
            let mut request = req(&pairs, mode, 2026);
            request.adaptive = adaptive;
            let plan = generate(&request, data).unwrap();
            println!("\n=== {mode} {pairs:?} adaptive={adaptive} ===");
            for section in plan.sections {
                println!("\n{}", section.name);
                for line in section.lines.iter().take(3) {
                    println!("{line}");
                }
            }
        }
    }
}
