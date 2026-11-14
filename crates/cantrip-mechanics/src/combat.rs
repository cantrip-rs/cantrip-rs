//! Combat recipes several cards print.

use cantrip_api::*;

/// "This creature can't be blocked by Walls" (CR 509.1b) as a `CardDef::block_rules` entry: a Wall, read through the source's own
/// text (CR 612), may block anything but it. Juggernaut and Bog Rats print it (Clockwork Swarm, Otarian Juggernaut and Rampart
/// Crawler too).
pub fn cant_be_blocked_by_walls(q: &dyn Query, source: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    source != attacker || !q.chars(blocker).has_subtype(q.words(source).subtype(Subtype::Wall))
}

/// "This creature can't be blocked except by Walls" / "can't be blocked by non-Wall creatures" (CR 509.1b) as a
/// `CardDef::block_rules` entry: only a Wall, read through the source's own text (CR 612), may block it. Flow of Maggots and
/// Evil Eye of Orms-by-Gore print it.
pub fn cant_be_blocked_except_by_walls(q: &dyn Query, source: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    source != attacker || q.chars(blocker).has_subtype(q.words(source).subtype(Subtype::Wall))
}

/// "This creature can't be blocked by creatures with power 3 or greater" (CR 509.1b): check current power
/// when blockers are declared. Amrou Kithkin and April O'Neil, Kunoichi Trainee print it.
pub fn power_three_or_greater_cant_block(q: &dyn Query, source: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    source != attacker || q.chars(blocker).power.unwrap_or(0) < 3
}

/// "This creature can't block creatures with power 2 or greater" (CR 509.1b): as the source, a blocker, would block an attacker
/// whose current power is 2 or more, as blockers are declared. Ironclaw Orcs and Brassclaw Orcs print it.
pub fn this_cant_block_power_two_or_greater(q: &dyn Query, source: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    source != blocker || q.chars(attacker).power.unwrap_or(0) < 2
}

/// "Creatures with power less than this creature's power can't block it" (CR 509.1b): of a possible block with the source as
/// the attacker, the blocker's power must be at least its own, both as they are as blockers are declared. A block that has been
/// made stands if powers change afterwards, since this is only asked when blocks are declared. Formation Breaker and Elusive
/// Otter print it.
pub fn power_less_than_its_cant_block_it(q: &dyn Query, source: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    source != attacker || q.chars(blocker).power.unwrap_or(0) >= q.chars(source).power.unwrap_or(0)
}

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

/// "Each creature you control with toughness greater than its power": `statics::creature_you_control`, with its toughness above its power as
/// it is asked. Ghalta the Immovable and Tapestry Warden print it for combat damage ("assigns combat damage equal to its
/// toughness rather than its power", CR 510.1a), and Tapestry Warden for stationing (CR 702.184c).
pub fn your_creature_with_toughness_greater_than_power(q: &dyn Query, asker: &Asker, creature: ObjectId) -> bool {
    let chars = q.chars(creature);
    crate::statics::creature_you_control(q, asker, creature) && chars.toughness.unwrap_or(0) > chars.power.unwrap_or(0)
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
            let ask = RequestKind::ChooseTarget { replacing: None, label: "Choose what it attacks as it enters attacking: a player, a planeswalker or a battle.", slot_player: None, together: None,
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

/// One `token` of `you`'s for each of `attacking`, attacking (tapped when `tapped`) what that entry names (CR 508.4): all of
/// them created together as one instruction (CR 101.4, 603.2c), whatever each attacks (`Game::create_tokens_attacking_each`).
/// The effect names no defender ("tapped and attacking"): each target was chosen with `choose_attack_target` before any
/// token is made, and a further token a replacement makes attacks what its controller chooses. Hero of Bladehold and The
/// Last Ronin's Technique print it.
pub fn create_attacking(g: &mut dyn Game, you: PlayerId, token: CardId, tapped: bool, attacking: &[Target]) -> Result<Vec<ObjectId>, Suspend> {
    let others = g.enter_attacking_targets(you);
    let entries: Vec<EntersAttacking> = attacking.iter().map(|&defender| EntersAttacking { defender, others: others.clone() }).collect();
    g.create_tokens_attacking_each(token, you, tapped, &entries)
}

/// "Enchanted creature ... can't be blocked by more than one creature" (CR 509.1b): `blocker_limits` rule of an
/// Aura, with no "has" a rule of the Aura's, not an ability the creature gains (CR 113.1a); a creature that loses all its
/// abilities is still under it (CR 613.1f). Alpha Authority and Meltstrider's Resolve print it.
pub fn one_blocker_at_most(q: &dyn Query, source: ObjectId, attacker: ObjectId) -> Option<usize> {
    (q.obj(source).attached_to == Some(attacker)).then_some(1)
}

/// "This creature can block any number of creatures" (CR 509.1a): a `block_capacities` rule of the creature's own. Wall of
/// Glare, Noble Ox, Palace Guard and Avatar of Hope print it.
pub fn can_block_any_number(_: &dyn Query, source: ObjectId, blocker: ObjectId) -> Option<BlockCapacity> {
    (source == blocker).then_some(BlockCapacity::AnyNumber)
}

/// "This creature can block an additional creature each combat" (CR 509.1a): a `block_capacities` rule of the creature's
/// own. Two-Headed Giant of Foriys, Foriysian Interceptor and Lairwatch Giant print it.
pub fn can_block_an_additional_creature(_: &dyn Query, source: ObjectId, blocker: ObjectId) -> Option<BlockCapacity> {
    (source == blocker).then_some(BlockCapacity::Additional(1))
}

/// Remove this blocker from combat; creatures it blocked that had become blocked by only this creature this combat
/// become unblocked (False Orders and Imprison). History matters: another blocker leaving earlier does not qualify
/// an attacker that had been blocked by two creatures (CR 509.1h, 509.3g).
pub fn remove_from_combat_and_unblock(g: &mut dyn Game, blocker: ObjectId) {
    let unblocked: Vec<ObjectId> = g.state().blocks.iter().filter(|b| b.blocker == blocker).map(|b| b.attacker).filter(|&attacker| {
        let mut seen = false;
        for record in crate::history::this_combat(g) {
            if let Event::Blocks { blocker: other, attacker: blocked, .. } = record.event {
                if blocked == attacker {
                    if other != blocker { return false; }
                    seen = true;
                }
            }
        }
        seen
    }).collect();
    g.remove_from_combat(blocker);
    g.end_sentence();
    instruction(g, |g| {
        for &attacker in &unblocked { g.become_unblocked(attacker); }
    });
}

/// Assign an existing creature to block a chosen attacker as an effect, rather than declaring it as a blocker.
/// False Orders and Balduvian Warlord print this instruction. The choice is not a target. The caller supplies the
/// attacking population and performs any preceding removal/unblocking instruction.
pub fn choose_attacker_to_block(g: &mut dyn Game, asker: &Asker, blocker: Option<ObjectId>, attackers: &[ObjectId]) -> Result<(), Suspend> {
    let Some(blocker) = blocker.filter(|&b| is_live(g,b)) else { return Ok(()) };
    let candidates = attackers.iter().copied().filter(|&a| is_live(g,a) && g.is_attacking(a)
        && g.defending_player(a) == Some(g.controller(blocker))).map(Target::Object).collect();
    if let Some(Target::Object(attacker)) = crate::choose::one_target(g,asker.controller,asker.source,candidates,"Choose an attacking creature to block")? {
        g.change_blocks(&[],&[],&[(blocker,attacker)]);
    }
    Ok(())
}

/// Exchange the blocking associations of two attackers (General Jarkeld) or of two blockers (Sorrow's Path).
/// Each instruction explicitly requires that the exchanged assignments could be made. Shared unchanged edges stay
/// shared for Jarkeld; Sorrow's Path removes both creatures from combat before assigning their new blocks.
pub fn swap_blocks(g: &mut dyn Game, first: Option<ObjectId>, second: Option<ObjectId>, attacking: bool) {
    let (Some(first),Some(second)) = (first,second) else { return };
    if first == second || !is_live(g,first) || !is_live(g,second) { return; }
    if attacking {
        if !g.is_attacking(first) || !g.is_attacking(second) || !g.is_blocked(first) || !g.is_blocked(second) { return; }
    } else if !g.is_blocking(first) || !g.is_blocking(second) { return; }
    let ending: Vec<_> = g.state().blocks.iter().filter(|b| {
        let id = if attacking { b.attacker } else { b.blocker }; id == first || id == second
    }).map(|b| (b.blocker,b.attacker)).collect();
    let starting: Vec<_> = ending.iter().map(|&(b,a)| {
        if attacking { (b,if a == first {second} else {first}) }
        else { (if b == first {second} else {first},a) }
    }).collect();
    if !g.block_reassignment_allowed(&ending,&starting) { return; }
    if attacking { g.change_blocks(&[],&ending,&starting); }
    else { g.change_blocks(&[first,second],&ending,&starting); }
}

/// "During combat" (CR 506.1): any step of the combat phase, beginning of combat through end of combat. Yuriko, Blade of the
/// Mighty ("During combat, players can't cast spells") and Jade Statue print it.
pub fn during_combat(q: &dyn Query) -> bool {
    matches!(
        q.state().turn.step(),
        Step::BeginCombat | Step::DeclareAttackers | Step::DeclareBlockers | Step::CombatDamage | Step::EndOfCombat
    )
}

/// "Cast this spell only during combat before blockers are declared" (CR 601.3): from the beginning of combat step through
/// the declare attackers step, of any combat phase (the 2013 ruling). Blaze of Glory, Disharmony, Gorilla War Cry, Panic and
/// Winter's Chill print it.
pub fn cast_only_before_blockers(q: &dyn Query, _: &Asker) -> bool {
    matches!(q.state().turn.step(), Step::BeginCombat | Step::DeclareAttackers)
}

/// "Activate only during combat" (CR 602.5b, 506.1): the activation condition form of `during_combat`. Jade Statue prints it.
pub fn activate_only_during_combat(q: &dyn Query, _: &Asker) -> bool {
    during_combat(q)
}

/// "Cast this spell / activate only before the combat damage step" (CR 506.8d, 506.8e, 506.8g): any step of the turn up to the
/// combat damage step, but only of the turn's first combat phase, so not once an end of combat step has begun (a second
/// combat phase, as Aurelia, the Warleader's, is too late), even when the damage step of the first was skipped. Berserk and
/// Angus Mackenzie print it, as a cast restriction (`CardDef::cast_only_if`) and an activation condition.
pub fn only_before_combat_damage_step(q: &dyn Query, _: &Asker) -> bool {
    q.state().turn.step() < Step::CombatDamage
        && !q.history().this_turn().iter().any(|r| matches!(r.event, Event::StepBegan { step: Step::EndOfCombat }))
}

/// "Activate only before attackers are declared" (CR 506.8a, 506.8d): the turn has not reached the declare attackers step, and
/// none has begun this turn (so not in a second combat phase's beginning of combat step). Arcum's Whistle and Norritt print it
/// (and it is the second half of `during_opponents_turn_before_attackers`).
pub fn before_attackers_are_declared(q: &dyn Query, _: &Asker) -> bool {
    q.state().turn.step() < Step::DeclareAttackers
        && !q.history().this_turn().iter().any(|r| matches!(r.event, Event::StepBegan { step: Step::DeclareAttackers }))
}

/// "Cast this spell / activate only during an opponent's turn, before attackers are declared" (CR 506.8a, 506.8d, 506.8e,
/// 506.8g): the active player is an opponent of the controller and `before_attackers_are_declared`. Nettling Imp and Siren's
/// Call print it.
pub fn during_opponents_turn_before_attackers(q: &dyn Query, asker: &Asker) -> bool {
    q.state().opponents(asker.controller).any(|p| p == q.state().turn.active) && before_attackers_are_declared(q, asker)
}

fn compel_target_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    crate::targets::permanent(q, id)
        && q.chars(id).is(CardType::Creature)
        && !q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Wall))
        && q.controller(id) == q.state().turn.active
        && q.obj(id).controlled_since < q.state().turn.number
}

/// "Target non-Wall creature the active player has controlled continuously since the beginning of the turn" (CR 115.1, 302.6).
/// Nettling Imp and Norritt print it; Arcum's Whistle words it its own way.
pub const COMPEL_TARGET: TargetSpec = target(compel_target_filter);

fn compel_must_attack(_: &dyn Query, _: &Asker, about: Target, id: ObjectId) -> bool {
    about.object() == Some(id)
}

/// "That creature attacks this turn if able" (CR 508.1d, 611.2a): the card's `lasting_rules` entry, started by
/// `must_attack_and_destroy_if_it_doesnt`. Nettling Imp and Norritt print it; Arcum's Whistle words it its own way.
pub const MUST_ATTACK_THIS_TURN: LastingRule =
    LastingRule { text: "That creature attacks this turn if able.", rule: LastingRuleKind::MustAttack, applies: compel_must_attack };

/// The delayed body of `DESTROY_IT_IF_IT_DIDNT_ATTACK` (CR 701.8): destroy the creature unless it attacked this turn. Arcum's
/// Whistle words its clause differently and declares this body with its own text.
pub fn destroy_if_it_didnt_attack(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject
        && !g.history().attacked_this_turn(creature)
    {
        g.destroy(creature);
    }
    Ok(())
}

/// "Destroy it at the beginning of the next end step if it didn't attack this turn" (CR 603.7, 701.8): the card's `delayed`
/// ability number 0, set up by `must_attack_and_destroy_if_it_doesnt`. Nettling Imp and Norritt print it; Arcum's Whistle words it its own way.
pub const DESTROY_IT_IF_IT_DIDNT_ATTACK: AbilityDef =
    delayed_trigger(destroy_if_it_didnt_attack, "Destroy it at the beginning of the next end step if it didn't attack this turn.");

/// "That creature attacks this turn if able. Destroy it at the beginning of the next end step if it didn't attack this turn"
/// (CR 508.1d, 603.7) about `creature`: starts `MUST_ATTACK_THIS_TURN` (lasting rule 0) and sets up
/// `DESTROY_IT_IF_IT_DIDNT_ATTACK` (delayed ability 0). Nettling Imp and Norritt print it; Arcum's Whistle words it its own way.
pub fn must_attack_and_destroy_if_it_doesnt(g: &mut dyn Game, item: &StackItem, creature: ObjectId) {
    g.add_lasting_rule(item, 0, Target::Object(creature), Duration::EndOfTurn);
    g.end_sentence();
    crate::delayed::at_next_end_step(g, item, 0, Target::Object(creature));
}

/// "Choose target [compel target]. That creature attacks this turn if able. Destroy it at the beginning of the next end step if
/// it didn't attack this turn." as an ability's effect, over `COMPEL_TARGET`. Nettling Imp and Norritt print it.
pub fn compel(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = crate::targets::object(g, item, 0) else { return Ok(()) };
    must_attack_and_destroy_if_it_doesnt(g, item, creature);
    Ok(())
}
