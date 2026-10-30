//! Combat recipes several cards print.

use cantrip_api::*;

/// "[This creature] can't be blocked this turn except by creatures with haste" (CR 509.1b): the restriction on the creature `on`,
/// as an attacker, that only a creature with haste as blockers are declared may block it. It says nothing of its blocking, nor of
/// a block already made. A card lists `EXCEPT_BY_HASTE` in its `lasting_block_rules` and the resolved ability adds
/// `EffectKind::BlockRestriction` for it. Gingerbrute and Speed, Young Avenger print it.
pub fn except_by_haste(q: &dyn Query, on: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    on != attacker || q.chars(blocker).has(Keyword::Haste)
}

/// The `LastingBlockRule` of `except_by_haste`.
pub const EXCEPT_BY_HASTE: LastingBlockRule = LastingBlockRule { text: "Can't be blocked except by creatures with haste", allows: except_by_haste };

/// "An unblocked attacking creature you control" (CR 509.1h): attacking, and no blocker was ever declared against it. What
/// ninjutsu (CR 702.49a) and sneak (CR 702.190a) return to their owner's hand.
pub fn unblocked_attacker_you_control(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.controller(id) == asker.controller && q.is_attacking(id) && !q.is_blocked(id)
}

/// What a creature that enters attacking attacks when its effect names nothing (CR 508.4): its controller chooses among
/// `Query::enter_attacking_targets`, and a lone candidate is taken without asking. None when there is nothing to attack. Raph & Mikey,
/// Troublemakers and Gut, True-Soul Zealot print it.
pub fn choose_attack_target(g: &mut dyn Game, you: PlayerId, source: ObjectId) -> Result<Option<Target>, Suspend> {
    let candidates = g.enter_attacking_targets(you);
    choose_among(g, you, source, &candidates)
}

/// `choose_attack_target` among these (CR 508.4): the controller chooses what a creature that enters attacking attacks, out
/// of `candidates`; a lone candidate is taken without asking, and None when there is none.
pub fn choose_among(g: &mut dyn Game, you: PlayerId, source: ObjectId, candidates: &[Target]) -> Result<Option<Target>, Suspend> {
    Ok(match candidates.len() {
        0 => None,
        1 => Some(candidates[0]),
        _ => {
            let ask = RequestKind::ChooseTarget { label: "Choose what it attacks as it enters attacking: a player, a planeswalker or a battle.", slot_player: None, together: None,
                source,
                purpose: Purpose::EnterAttacking,
                candidates: candidates.to_vec(),
                optional: false,
                least: 1,
                count: 1,
                any_number: false,
            };
            match g.ask(you, ask)? {
                Answer::Targets(t) => t.first().copied().or(Some(candidates[0])),
                _ => Some(candidates[0]),
            }
        }
    })
}

/// `count` of `token`, each attacking (tapped when `tapped`) what its controller chooses for it (CR 508.4): `choose_attack_target` once per
/// token, asked only when there is a choice and before any token is made, and the ones that attack the same thing created
/// together (CR 101.4). Nothing is made when there is nothing to attack. Hero of Bladehold and Dalkovan Encampment print it
/// with two, Otharri, Suns' Glory with one for each experience counter, all tapped; Parhelion II's Angels are not.
pub fn create_attacking_of_your_choice(g: &mut dyn Game, you: PlayerId, source: ObjectId, token: CardId, count: usize, tapped: bool) -> Result<Vec<ObjectId>, Suspend> {
    let mut chosen: Vec<Target> = Vec::new();
    for _ in 0..count {
        match choose_attack_target(g, you, source)? {
            Some(target) => chosen.push(target),
            None => break,
        }
    }
    create_attacking(g, you, token, tapped, &chosen)
}

/// A player and the planeswalkers they control: what "that player or a planeswalker they control" can be attacked as
/// (CR 508.4, 508.5). Adeline, Resplendent Cathar prints it.
pub fn player_or_their_planeswalkers(g: &dyn Game, player: PlayerId) -> Vec<Target> {
    let state = g.state();
    let mut out = vec![Target::Player(player)];
    out.extend(
        state
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| state.obj(id).controller == player && g.chars(id).is(CardType::Planeswalker))
            .map(Target::Object),
    );
    out
}

/// One `token` of `you`'s for each of `attacking`, attacking (tapped when `tapped`) what that entry names (CR 508.4): the tokens that
/// attack the same thing are created together (CR 101.4). The chosen targets come from `choose_among` or
/// `choose_attack_target`, asked before any token is made. Hero of Bladehold and Adeline, Resplendent Cathar print it.
pub fn create_attacking(g: &mut dyn Game, you: PlayerId, token: CardId, tapped: bool, attacking: &[Target]) -> Result<Vec<ObjectId>, Suspend> {
    let mut made = Vec::new();
    let mut done: Vec<Target> = Vec::new();
    for &target in attacking {
        if done.contains(&target) {
            continue;
        }
        done.push(target);
        let n = attacking.iter().filter(|&&t| t == target).count();
        let tokens = vec![(token, you); n];
        made.extend(g.create_tokens_attacking(&tokens, tapped, target)?);
    }
    Ok(made)
}
