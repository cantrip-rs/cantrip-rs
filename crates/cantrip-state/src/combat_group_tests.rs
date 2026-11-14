//! Compare declaration validation and completion against complete small games,
//! using the three printed restrictions as an independent legality oracle.
use crate::*;
const IDS: [ObjectId; 3] = [ObjectId(10), ObjectId(11), ObjectId(12)];
const TARGETS: [Target; 2] = [Target::Player(PlayerId(1)), Target::Player(PlayerId(2))];
fn groups(flags: u32) -> Vec<CombatGroup> {
    let mut out = vec![];
    if flags & 1 != 0 {
        out.push(CombatGroup { subjects: IDS.to_vec(), members: IDS.to_vec(), at_least: 0, at_most: 2 });
    }
    if flags & 2 != 0 {
        out.push(CombatGroup { subjects: vec![IDS[0]], members: IDS[1..].to_vec(), at_least: 2, at_most: u32::MAX });
    }
    if flags & 4 != 0 {
        out.push(CombatGroup { subjects: vec![IDS[1]], members: IDS.to_vec(), at_least: 0, at_most: 1 });
    }
    out
}
fn printed_allows(flags: u32, present: impl Fn(ObjectId) -> bool) -> bool {
    let n = IDS.iter().filter(|&&id| present(id)).count();
    // Caverns: at most two; Conscripts: only with two others; Errantry: only alone.
    (flags & 1 == 0 || n <= 2) && (flags & 2 == 0 || !present(IDS[0]) || n == 3) && (flags & 4 == 0 || !present(IDS[1]) || n == 1)
}
fn declarations() -> Vec<Vec<(ObjectId, Target)>> {
    (0..27)
        .map(|mut n| {
            IDS.iter()
                .filter_map(|&id| {
                    let choice = n % 3;
                    n /= 3;
                    (choice > 0).then(|| (id, TARGETS[choice - 1]))
                })
                .collect()
        })
        .collect()
}
#[test]
fn combat_group_attacks_match_exhaustive_requirements_costs_caps_and_completions() {
    let all = declarations();
    for flags in 1..8 {
        for capped in [false, true] {
            for paid in [false, true] {
                let mut r = AttackRules {
                    groups: groups(flags),
                    required: IDS
                        .iter()
                        .map(|&attacker| RequiredAttack { attacker, aim: AttackAim::Anything })
                        .chain([RequiredAttack { attacker: IDS[0], aim: AttackAim::PlayerOtherThan(PlayerId(1)) }])
                        .collect(),
                    barred: vec![(IDS[2], TARGETS[1])],
                    limits: if capped { vec![(TARGETS[0], 1)] } else { vec![] },
                    ..Default::default()
                };
                let costs = if paid {
                    vec![AttackCost { attacker: IDS[1], target: TARGETS[0], cost: ManaCost { generic: 1, ..ManaCost::ZERO }, source: IDS[1] }]
                } else {
                    vec![]
                };
                let valid = |d: &Vec<(ObjectId, Target)>| {
                    printed_allows(flags, |id| d.iter().any(|p| p.0 == id))
                        && !d.contains(&(IDS[2], TARGETS[1]))
                        && (!capped || d.iter().filter(|p| p.1 == TARGETS[0]).count() <= 1)
                };
                let score = |d: &Vec<(ObjectId, Target)>| d.len() as u32 + u32::from(d.contains(&(IDS[0], TARGETS[1])));
                let free = |d: &Vec<(ObjectId, Target)>| !paid || !d.contains(&(IDS[1], TARGETS[0]));
                let most = all.iter().filter(|d| valid(d) && free(d)).map(score).max().unwrap();
                (r.most, r.suggested) = r.solve(&IDS, &TARGETS, &costs);
                assert_eq!(r.most, most, "flags={flags} capped={capped} paid={paid}");
                assert!(valid(&r.suggested) && free(&r.suggested));
                assert_eq!(score(&r.suggested), most);
                for partial in &all {
                    let verdict = r.judge(&IDS, &TARGETS, &costs, partial, true);
                    assert_eq!(verdict.complete, valid(partial) && score(partial) >= most, "flags={flags} partial={partial:?}");
                    let expected: Vec<_> = IDS
                        .iter()
                        .flat_map(|&id| TARGETS.map(|t| (id, t)))
                        .filter(|p| {
                            !partial.iter().any(|q| q.0 == p.0)
                                && all.iter().any(|d| valid(d) && score(d) >= most && d.contains(p) && partial.iter().all(|p| d.contains(p)))
                        })
                        .collect();
                    assert_eq!(verdict.addable, expected, "flags={flags} partial={partial:?}");
                }
            }
        }
    }
}
#[test]
fn combat_group_blocks_count_distinct_creatures_and_match_every_completion() {
    let attackers = [ObjectId(100), ObjectId(101)];
    let allowed: Vec<_> = IDS.iter().flat_map(|&b| attackers.map(|a| (b, a))).collect();
    let all: Vec<Vec<_>> = (0..64).map(|mask| allowed.iter().enumerate().filter(|(i, _)| mask & (1 << i) != 0).map(|(_, p)| *p).collect()).collect();
    for flags in 1..4 {
        for multi in [false, true] {
            for paid in [false, true] {
                let mut r = BlockRequirements {
                    groups: groups(flags),
                    required: IDS
                        .iter()
                        .map(|&b| RequiredBlock { blocker: Some(b), attacker: None })
                        .chain([RequiredBlock { blocker: Some(IDS[0]), attacker: Some(attackers[1]) }])
                        .collect(),
                    capacities: if multi { vec![(IDS[0], 2)] } else { vec![] },
                    limits: vec![(attackers[0], 2, 3)],
                    costs: if paid {
                        vec![BlockCost { blocker: IDS[1], attacker: attackers[0], cost: ManaCost { generic: 1, ..ManaCost::ZERO }, source: IDS[1] }]
                    } else {
                        vec![]
                    },
                    ..Default::default()
                };
                let valid = |d: &Vec<(ObjectId, ObjectId)>| {
                    printed_allows(flags, |id| d.iter().any(|p| p.0 == id))
                        && IDS.iter().all(|&b| d.iter().filter(|p| p.0 == b).count() <= if multi && b == IDS[0] { 2 } else { 1 })
                        && d.iter().filter(|p| p.1 == attackers[0]).count() != 1
                };
                let score = |d: &Vec<(ObjectId, ObjectId)>| {
                    IDS.iter().filter(|&&b| d.iter().any(|p| p.0 == b)).count() as u32 + u32::from(d.contains(&(IDS[0], attackers[1])))
                };
                let free = |d: &Vec<(ObjectId, ObjectId)>| !paid || !d.contains(&(IDS[1], attackers[0]));
                let most = all.iter().filter(|d| valid(d) && free(d)).map(score).max().unwrap();
                let (got, suggested) = r.solve(&allowed, &[], None, true).unwrap();
                r.most = got;
                assert_eq!(got, most, "flags={flags} multi={multi} paid={paid}");
                assert!(valid(&suggested) && free(&suggested));
                assert_eq!(score(&suggested), most);
                for partial in &all {
                    let verdict = r.judge(&allowed, partial, true);
                    assert_eq!(verdict.complete, valid(partial) && score(partial) >= most, "flags={flags} partial={partial:?}");
                    let expected: Vec<_> = allowed
                        .iter()
                        .copied()
                        .filter(|p| {
                            !partial.contains(p)
                                && all.iter().any(|d| valid(d) && score(d) >= most && d.contains(p) && partial.iter().all(|p| d.contains(p)))
                        })
                        .collect();
                    assert_eq!(verdict.addable, expected, "flags={flags} partial={partial:?}");
                }
            }
        }
    }
}

#[test]
fn a_global_cap_with_many_required_attackers_keeps_the_search_bounded() {
    let ids: Vec<_> = (0..128).map(|n| ObjectId(1000 + n)).collect();
    let r = AttackRules {
        groups: vec![CombatGroup { subjects: ids.clone(), members: ids.clone(), at_least: 0, at_most: 2 }],
        required: ids.iter().map(|&attacker| RequiredAttack { attacker, aim: AttackAim::Anything }).collect(),
        ..Default::default()
    };
    crate::attacks::GROUP_ATTACK_SEARCH_NODES.with(|n| n.set(0));
    let (most, d) = r.solve(&ids, &TARGETS, &[]);
    assert_eq!(most, 2);
    assert_eq!(d.len(), 2);
    let nodes = crate::attacks::GROUP_ATTACK_SEARCH_NODES.with(|n| n.get());
    assert!(nodes < 1000, "global cap visited {nodes} nodes");
}
