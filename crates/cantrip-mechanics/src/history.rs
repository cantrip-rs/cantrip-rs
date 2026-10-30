//! The questions about the past that several printed cards ask in the same words, written once over the event
//! history (`cantrip_api::history`). A trigger matching an event and a replacement deciding about one both see the
//! history as it was just before it, so each of these reads the same from either.

use cantrip_api::*;

/// "A creature that dealt damage this turn" (CR 120.2): `source` dealt damage this turn, to anything, combat or not. Damage that
/// was prevented, or whose amount was zero, was not dealt (CR 615.6). The object is the one that dealt it, so a permanent that left
/// and came back is another (CR 400.7). Red Guardian, Super-Soldier, Avenging Arrow and Executioner's Swing print it.
pub fn dealt_damage_this_turn(history: History<'_>, source: ObjectId) -> bool {
    history.damage_dealt().any(|d| d.source == source && d.amount > 0)
}

/// An opponent was dealt noncombat damage this turn (CR 120.2). Grim Repriser and Whiplash Wordsmith ask this
/// same question, from an activated ability's condition and a static ability's layers respectively.
pub fn opponent_dealt_noncombat_damage_this_turn(history: History<'_>, state: &GameState, player: PlayerId) -> bool {
    history.this_turn().iter().any(|r| {
        matches!(r.event, Event::Damage { target: Target::Player(p), amount, kind, .. }
            if !kind.is_combat() && amount > 0 && state.opponents(player).any(|opponent| opponent == p))
    })
}

/// Whether a draw `player` is about to make (or is making, from a trigger on it) is the first card they draw in their
/// own draw step this turn: the draw that "except the first one they draw in each of their draw steps" exempts. Orcish
/// Bowmasters (a trigger) and Hullbreacher (a replacement) print it.
pub fn first_draw_in_own_draw_step(q: &dyn Query, player: PlayerId) -> bool {
    let turn = q.state().turn;
    turn.active == player
        && turn.step() == Step::Draw
        && !q.history().any(EventQuery::kind(EventMask::DREW).player(player).this_turn().step(Step::Draw))
}

/// Cards `player` has discarded this turn (CR 701.9), as the game is now: "for each card you've discarded this turn", a discard
/// made as a cost or earlier in the same resolution included. Jiang Yanggu, Alone and Misty Knight, Hero for Hire print it.
pub fn cards_discarded_this_turn(q: &dyn Query, player: PlayerId) -> u32 {
    q.history().count(EventQuery::kind(EventMask::DISCARDED).player(player).this_turn()) as u32
}

/// Cards `player` has drawn this turn, before the draw being asked about (Faerie Mastermind's "second card", Leovold's
/// and Narset's "more than one card each turn").
pub fn drawn_this_turn(q: &dyn Query, player: PlayerId) -> usize {
    q.history().drawn(player)
}

/// "Draws their second card each turn" (CR 120.3, 603.2): the player making this draw, if it is their second card this
/// turn, whichever step it falls in (the history holds the draws before the one being matched). Faerie Mastermind ("an
/// opponent") and King T'Challa ("a player") print it.
pub fn second_card_drawn_this_turn(q: &dyn Query, event: &Event) -> Option<PlayerId> {
    match event {
        Event::Drew { player, .. } if drawn_this_turn(q, *player) == 1 => Some(*player),
        _ => None,
    }
}

/// Spells cast this turn by every player (storm, CR 702.40a, counts those cast before the storm spell: subtract it).
pub fn spells_cast_this_turn(q: &dyn Query) -> usize {
    q.history().casts().count()
}

/// Revolt (CR 207.2c): whether a permanent `player` controlled left the battlefield this turn. The leaving object's
/// row is as it was when it left, so its controller then is read from it.
pub fn permanent_left_battlefield_this_turn(q: &dyn Query, player: PlayerId) -> bool {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).from(ZoneKind::Battlefield).this_turn())
        .any(|r| matches!(r.event, Event::ZoneChange { old, .. } if q.obj(old).controller == player))
}

/// The number of permanents `player` has sacrificed this turn (CR 701.21a): every permanent that went from the battlefield to a
/// graveyard as a sacrifice this turn, token or not, under that player's control as it left. Asked of the history (CR 603.10a).
/// Obsessive Pursuit counts them; Phoenix Fleet Airship asks whether there was one ("if you sacrificed a permanent this turn").
pub fn permanents_sacrificed_this_turn(q: &dyn Query, player: PlayerId) -> usize {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).from(ZoneKind::Battlefield).this_turn())
        .filter(|r| crate::triggers::sacrificed_by(q, player, &r.event).is_some())
        .count()
}

/// The void condition: "a nonland permanent left the battlefield this turn or a spell was warped this turn". Void is an
/// ability word (CR 207.2c) with no rules of its own; only its second clause has a rule, CR 702.185c: a spell was warped when
/// it was cast for its warp cost. Any player's permanent counts, a token too, judged as it was as it left the battlefield (CR 608.2h). Elegy Acolyte, Plasma Bolt and
/// every other card with a void ability print it.
pub fn void(q: &dyn Query) -> bool {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).from(ZoneKind::Battlefield).this_turn())
        .any(|r| matches!(r.event, Event::ZoneChange { old, .. } if q.state().last_known(old).is_some_and(|l| !l.types.has(CardType::Land))))
        || crate::warp::spell_was_warped_this_turn(q)
}

/// Cards (not tokens) that left `player`'s graveyard this turn (Gau, Feral Youth).
pub fn cards_left_graveyard_this_turn(q: &dyn Query, player: PlayerId) -> usize {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).from(ZoneKind::Graveyard).this_turn())
        .filter(|r| matches!(r.event, Event::ZoneChange { old, from, .. } if from == Zone::graveyard(player) && !q.obj(old).token))
        .count()
}

/// The cards put into `owner`'s graveyard from the battlefield this turn that are still there (Faith's Reward, Second
/// Sunrise, No Rest for the Wicked): the graveyard objects those moves made, in the order they arrived.
pub fn put_into_graveyard_from_battlefield_this_turn(q: &dyn Query, owner: PlayerId) -> Vec<ObjectId> {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).from(ZoneKind::Battlefield).to(ZoneKind::Graveyard).this_turn())
        .filter_map(|r| match r.event {
            // A token that left the battlefield is not a card in the graveyard (CR 111.7, 704.5d).
            Event::ZoneChange { old, new, to, .. }
                if to == Zone::graveyard(owner) && q.obj(new).became.is_none() && !q.obj(old).token =>
            {
                Some(new)
            }
            _ => None,
        })
        .collect()
}

/// Whether `permanent` entered the battlefield this turn (CR 400.7: it is the object that move made). Asked of a history
/// so that a static ability's condition reads it too (`LayerQuery::history`): Shardmage's Rescue's "as long as this Aura
/// entered this turn", The Wandering Emperor's, power-up's cost.
pub fn entered_this_turn(history: History<'_>, permanent: ObjectId) -> bool {
    history
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).to(ZoneKind::Battlefield).this_turn())
        .any(|r| matches!(r.event, Event::ZoneChange { new, .. } if new == permanent))
}

/// How many of `player`'s opponents have lost life this turn (spectacle's "an opponent lost life this turn", Drill
/// Bit; Kaito, Bane of Nightmares counts them).
pub fn opponents_who_lost_life_this_turn(q: &dyn Query, player: PlayerId) -> usize {
    let h = q.history();
    q.state().opponents(player).filter(|&p| h.life_lost(p) > 0).count()
}

/// Whether an opponent of `player` has lost life this turn (CR 119.3: "lost life", not a lower life total - one who gained 2
/// and lost 1 still did). Spectacle, Fireglass Mentor and Flamecache Gecko ask it.
pub fn an_opponent_lost_life_this_turn(q: &dyn Query, player: PlayerId) -> bool {
    opponents_who_lost_life_this_turn(q, player) > 0
}

/// `an_opponent_lost_life_this_turn` as a condition of the ability's controller: a triggered ability's intervening "if an
/// opponent lost life this turn" (CR 603.4) - Fireglass Mentor, Flamecache Gecko - and spectacle's.
pub fn an_opponent_lost_life(q: &dyn Query, asker: &Asker) -> bool {
    an_opponent_lost_life_this_turn(q, asker.controller)
}

/// Whether `object` arrived where it is this turn (CR 400.7: it is the object a move this turn made), in whatever zone:
/// Night Nurse's "put there from anywhere this turn", and, on the battlefield, `entered_this_turn`.
pub fn arrived_this_turn(q: &dyn Query, object: ObjectId) -> bool {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).this_turn())
        .any(|r| matches!(r.event, Event::ZoneChange { new, .. } if new == object))
}

/// The creatures that died this turn (CR 700.4): every move from the battlefield to a graveyard this turn of an object
/// that was a creature as it left, by last known information, so a token that has since ceased to exist still counts.
/// With `under`, only those that player controlled as they died (Callous Sell-Sword's "under your control"; the leaving
/// object's row is as it was when it left). Spymaster's Vault counts them all.
pub fn creatures_died_this_turn(q: &dyn Query, under: Option<PlayerId>) -> usize {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).from(ZoneKind::Battlefield).to(ZoneKind::Graveyard).this_turn())
        .filter(|r| {
            matches!(r.event, Event::ZoneChange { old, .. }
                if q.state().last_known(old).is_some_and(|l| l.types.has(CardType::Creature))
                    && under.is_none_or(|p| q.obj(old).controller == p))
        })
        .count()
}

/// "If you gained life this turn" (CR 603.4, 119.3): whether `player` has gained any life this turn. Ocelot Pride's and
/// Enlightened Confidant's intervening ifs print it.
pub fn gained_life_this_turn(q: &dyn Query, player: PlayerId) -> bool {
    q.history().life_gained(player) > 0
}

/// `gained_life_this_turn` as a condition of the ability's controller: a triggered ability's intervening "if you gained life this
/// turn" (CR 603.4). Lunar Convocation and Moseo, Vein's New Dean print it.
pub fn you_gained_life_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    gained_life_this_turn(q, asker.controller)
}

/// "If you've gained 5 or more life this turn" (CR 603.4, 119.3): whether the ability's controller has gained at least five life
/// this turn, as an intervening "if" or a "to solve" condition. Resplendent Angel and Case of the Uneaten Feast print it.
pub fn you_gained_five_or_more_life_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    q.history().life_gained(asker.controller) >= 5
}

/// "If you lost life this turn" (CR 603.4, 119.3): whether `player` has lost any life this turn, however much they also gained
/// (damage and paying life count). Lunar Convocation and Thought-Stalker Warlock print it.
pub fn lost_life_this_turn(q: &dyn Query, player: PlayerId) -> bool {
    q.history().life_lost(player) > 0
}

/// Whether `player` has activated an equip ability this turn (CR 702.6a, 602.2): what "the first equip ability you activate
/// [during each of your turns / each turn]" asks of an equip ability being announced, which is not yet in the history.
/// Forge Anew and Kíli the Resourceful print it.
pub fn activated_an_equip_ability_this_turn(q: &dyn Query, player: PlayerId) -> bool {
    q.history().this_turn().iter().any(|r| {
        matches!(r.event, Event::AbilityActivated { ability, player: p, .. }
            if p == player && q.activated_ability(ability).is_some_and(|d| d.equip))
    })
}

/// "Choose one that hasn't been chosen" (CR 700.2): whether mode `mode` of a modal ability has not been chosen before by the
/// object that has it (`asker.source`), as its ability went on the stack. A mode chosen for an ability that was then countered
/// or fizzled stays chosen, whoever controlled the object when it was chosen, and an object that left and came back is a new one
/// (CR 400.7) with every mode to choose again. Gollum, Riddle Master and Demonic Pact print it (`ModalDef::allowed`).
pub fn mode_not_chosen_before(q: &dyn Query, asker: &Asker, mode: u8) -> bool {
    !q.history().all().iter().any(|r| {
        matches!(r.event, Event::ModesChosen { source, modes, .. } if source == asker.source && modes & (1 << mode) != 0)
    })
}

/// "Choose one that hasn't been chosen this turn" (CR 700.2): whether mode `mode` has not been chosen this turn by the object
/// that has the ability (`asker.source`), as its ability went on the stack. A mode chosen for an ability that was then
/// countered stays chosen, and an object that left and came back is a new one (CR 400.7) with its own. With every mode chosen
/// the ability still triggers and is removed as it would be put on the stack (CR 603.3c). Monument to Endurance and The Vision
/// print it (`ModalDef::allowed`).
pub fn mode_not_chosen_this_turn(q: &dyn Query, asker: &Asker, mode: u8) -> bool {
    !q.history().this_turn().iter().any(|r| {
        matches!(r.event, Event::ModesChosen { source, modes, .. } if source == asker.source && modes & (1 << mode) != 0)
    })
}

/// "The sacrificed creature's power" (CR 601.2h, 608.2h): the power of what `spell`'s additional cost sacrificed, as it last
/// existed on the battlefield, pumps, counters and copy effects included. The sacrifice is the last one from the battlefield
/// in the history before the spell was cast. A copy of the spell was never cast: it reads what the original sacrificed (CR
/// 707.10, whose example is Fling), through a copy of a copy too. Kazuul's Fury and Fling print it.
pub fn sacrificed_creatures_power(q: &dyn Query, spell: ObjectId) -> i32 {
    let records = q.history().all();
    let spell = q.history().original_spell(spell);
    let Some(cast) = records.iter().rposition(|r| matches!(r.event, Event::Cast(c) if c.spell == spell)) else { return 0 };
    records[..cast]
        .iter()
        .rev()
        .find_map(|r| match r.event {
            Event::ZoneChange { old, from, reason: ZoneReason::Sacrifice, .. } if from.is(ZoneKind::Battlefield) => Some(old),
            _ => None,
        })
        .map_or(0, |creature| q.chars(creature).power.unwrap_or(0) as i32)
}

/// "That creature's power" of a creature a resolving ability sacrificed (CR 701.21, 608.2h): the power of the permanent that
/// `sacrificed` (the new object in the graveyard, as `effects::may_sacrifice` returns it) was before the sacrifice, as it last
/// existed on the battlefield, pumps, counters and copy effects included. It is the old object of the zone change that made
/// `sacrificed` (CR 400.7); zero for an object no zone change made. Bolg of the North and Rhovanion Rampager print it.
pub fn power_of_sacrificed_permanent(q: &dyn Query, sacrificed: ObjectId) -> i16 {
    q.history()
        .all()
        .iter()
        .rev()
        .find_map(|r| if let Event::ZoneChange { old, new, .. } = r.event && new == sacrificed { Some(old) } else { None })
        .map_or(0, |old| crate::targets::power_or_last_known(q, old))
}

/// "Each creature that convoked it" (CR 702.51c, 109.2): the creatures that convoked `object` (a spell, or the permanent it
/// became) that are creatures on the battlefield now. A convoker that has since left the battlefield is a new object
/// wherever it went (CR 400.7), and one that is no longer a creature is not a creature that convoked it (CR 109.2: March of
/// the Machines ended), so nothing happens to either. Venerated Loxodon and Zephyr Singer print it.
pub fn convoking_creatures_on_the_battlefield(q: &dyn Query, object: ObjectId) -> Vec<ObjectId> {
    q.history()
        .convoked(object)
        .filter(|&creature| is_live(q, creature) && q.obj(creature).zone.is(ZoneKind::Battlefield) && q.chars(creature).is(CardType::Creature))
        .collect()
}

/// "Your first, second, or third turn of the game" (CR 500, 102.1): it is `player`'s own turn and they have begun at most `n`
/// turns, an extra turn counting as one. Starting Town's enters-tapped clause and Jace Reawakened's casting restriction print it
/// (with `n` = 3).
pub fn in_your_first_turns(q: &dyn Query, player: PlayerId, n: u32) -> bool {
    q.state().turn.active == player
        && q.history().select(EventQuery::kind(EventMask::TURN_BEGAN).player(player)).nth(n as usize).is_none()
}

/// "If you've completed a dungeon" (CR 309.7): whether `player` has completed one this game. Caves of Chaos Adventurer,
/// Undermountain Adventurer and White Plume Adventurer print it.
pub fn completed_a_dungeon(q: &dyn Query, player: PlayerId) -> bool {
    q.history().any(EventQuery::kind(EventMask::DUNGEON_COMPLETED).player(player))
}

/// The first noncreature spell a player casts this turn (CR 603.2), including spells cast before
/// a permanent entered. Plan for All Outcomes and The Queen of Dale ask this same history question.
pub fn first_noncreature_spell_this_turn(q: &dyn Query, spell: ObjectId, player: PlayerId) -> bool {
    !q.history().casts_by(player).any(|c| c.spell != spell && !c.is(CardType::Creature))
}

/// "If you've scried or surveilled this turn" (CR 701.22, 701.25): `player` completed a scry or a surveil this turn, of any
/// number of cards. Desperate Futurescribe and Proctor of Potential ask it.
pub fn scried_or_surveilled_this_turn(q: &dyn Query, player: PlayerId) -> bool {
    q.history()
        .this_turn()
        .iter()
        .any(|r| matches!(r.event, Event::Scried { player: p, .. } | Event::Surveilled { player: p, .. } if p == player))
}

/// Whether the permission `source` gives has been used this turn: a spell cast or a land played by it, as the history
/// records what each was made by (`Cast::granted_by`, `Event::LandPlayed`). "Once during each of your turns, you may cast
/// ... from your graveyard" (CR 601.3): Lurrus of the Dream-Den, and Serra Paragon, whose land plays share the once. It is
/// that object's own: another permanent with the same ability is another permission, and one that left and came back is a
/// new object (CR 400.7).
pub fn permission_used_this_turn(q: &dyn Query, source: ObjectId) -> bool {
    q.history().this_turn().iter().any(|r| match r.event {
        Event::Cast(c) => c.granted_by().is_some_and(|g| g.object == source),
        Event::LandPlayed { granted_by, .. } => granted_by.some().is_some_and(|g| g.object == source),
        _ => false,
    })
}

/// The permanents `source` dealt damage to (CR 120.3) in the records from index `since` on (`history().head()` noted before
/// the damage), each once, in the order dealt: read from the damage events, so damage that was prevented names none and a
/// redirection (CR 614.9, Ancient Adamantoise) names the permanent the damage was dealt to, not the one aimed at. Anger of
/// the Gods, Redcap Melee, The Black Arrow and Torch the Tower print "dealt damage this way" about them.
pub fn permanents_dealt_damage_by(history: History, source: ObjectId, since: usize) -> Vec<ObjectId> {
    let mut dealt: Vec<ObjectId> = Vec::new();
    for r in &history.all()[since..] {
        if let Event::Damage { source: s, target: Target::Object(id), amount, .. } = r.event {
            if s == source && amount > 0 && !dealt.contains(&id) {
                dealt.push(id);
            }
        }
    }
    dealt
}

/// "If you cast this spell during your main phase" (CR 505.1, 601.2i): a `CardDef::cast_fact`, 1 if the spell's caster was
/// the active player in a precombat or postcombat main phase as it became cast, else 0. Read with
/// `history().fact_as_cast`, so a copy (not cast, CR 707.10) never has it. Tail Swipe, Might of Old Krosa, Haunting Hymn and
/// the other "Addendum" spells print it.
pub fn cast_during_your_main_phase(q: &dyn Query, me: &Asker) -> u32 {
    let turn = q.state().turn;
    (turn.active == me.controller && turn.step().is_main()) as u32
}

/// "Attacked with N creatures this turn" (CR 508.1): how many different creatures were declared as attackers this turn by
/// `player`, who declares them only on their own turn (CR 506.2, 508.1a), so any other player has attacked with none - an
/// ability's controller need not be the active player when a copy of it is controlled by another (CR 707.10). A creature
/// that attacked in two combats counts once, one put onto the battlefield attacking was never declared, and the creature
/// need not still be on the battlefield or under anyone's control. Windbrisk Heights, Wedding Announcement and Chart a
/// Course (whether it is any) ask it.
pub fn creatures_attacked_with(q: &dyn Query, player: PlayerId) -> usize {
    let mut seen: Vec<ObjectId> = Vec::new();
    for a in q.history().attacked_with(player) {
        if !seen.contains(&a.attacker) {
            seen.push(a.attacker);
        }
    }
    seen.len()
}

/// This combat's records (CR 506.1): those of this turn since its latest beginning of combat step began, so a second combat
/// phase in a turn (CR 500.8) has its own; empty before the turn's first combat. Melee's "each opponent you attacked this
/// combat" and Kytheon, Hero of Akros's "if Kytheon and at least two other creatures attacked this combat" ask it.
pub fn this_combat(q: &dyn Query) -> &[Record] {
    let turn = q.history().this_turn();
    match turn.iter().rposition(|r| matches!(r.event, Event::StepBegan { step: Step::BeginCombat })) {
        Some(began) => &turn[began..],
        None => &[],
    }
}

/// `cards_left_graveyard_this_turn` as a condition of the ability's controller: a triggered ability's intervening "if a card
/// left your graveyard this turn" (CR 603.4). Gau, Feral Youth and Primary Research print it.
pub fn a_card_left_your_graveyard_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    cards_left_graveyard_this_turn(q, asker.controller) > 0
}
