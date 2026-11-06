//! Player designations and the dungeon: the monarch (CR 725), the initiative (CR 726) and venturing into the Undercity
//! (CR 701.49, 309). Their abilities have no source object: the rules put them on the stack themselves, from an
//! object of the rules' own that is in no zone.

use cantrip_api::intrinsic::{INITIATIVE_TAKEN, INITIATIVE_VENTURE, MONARCH_DRAW, MONARCH_TAKEN};
use cantrip_api::*;
use cantrip_state::{PendingTrigger, Span};

use crate::{Cx, R};

/// Puts an ability of the rules' own on its way to the stack (CR 603.3), controlled by `controller` and about
/// `subject`. Its source is a new object in no zone: it ceases to matter once the ability resolves. The source.
pub(crate) fn queue(cx: &mut Cx, card: CardId, index: u8, controller: PlayerId, subject: Target) -> ObjectId {
    let source = cx.st().push_obj(Obj::new(card, controller, Zone::GONE));
    cx.st().trigger_push(PendingTrigger { event_amount: None,
        source,
        ability: AbilityRef { card, index },
        controller,
        subject,
        data: [0; 2],
        members: Span::default(),
        count: None,
        since: None,
        acquired: Acquired::OWN,
        words: None,
    });
    source
}

fn in_game(cx: &Cx, player: PlayerId) -> bool {
    !cx.s().player(player).lost
}

/// A player becomes the monarch (CR 725.1); whoever was stops being it (CR 725.3). Nothing happens to one who is it
/// already or who has left the game.
pub fn become_monarch(cx: &mut Cx, player: PlayerId) {
    if !in_game(cx, player) || cx.s().player(player).monarch {
        return;
    }
    set_monarch(cx, Some(player));
}

fn set_monarch(cx: &mut Cx, monarch: Option<PlayerId>) {
    for p in cx.s().seats().collect::<Vec<_>>() {
        let is = Some(p) == monarch;
        cx.st().update_player(p, |pl| pl.monarch = is);
    }
    // What watches a player becoming the monarch (Palace Jailer's wait) sees it (CR 725.1).
    if let Some(player) = monarch {
        cx.emit(Event::BecameMonarch { player });
    }
}

/// A monarch who has left the game hands the designation to the active player, or if that is who left, the next
/// player in turn order still in the game (CR 725.4).
pub fn monarch_left(cx: &mut Cx) {
    let Some(leaver) = cx.s().seats().find(|&p| cx.s().player(p).monarch && cx.s().player(p).lost) else { return };
    let order: Vec<PlayerId> = cx.s().apnap().collect();
    let heir = order.into_iter().find(|&p| p != leaver && in_game(cx, p));
    set_monarch(cx, heir);
}

/// The player with the initiative who has left the game hands it to the active player, or if that is who left, the next
/// player in turn order still in the game, who takes it (CR 726.4) - so ventures into Undercity as any taker does (CR 726.2).
pub fn initiative_left(cx: &mut Cx) {
    let Some(leaver) = cx.s().seats().find(|&p| cx.s().player(p).initiative && cx.s().player(p).lost) else { return };
    let order: Vec<PlayerId> = cx.s().apnap().collect();
    match order.into_iter().find(|&p| p != leaver && in_game(cx, p)) {
        Some(heir) => take_initiative(cx, heir),
        None => cx.st().update_player(leaver, |pl| pl.initiative = false),
    }
}

/// The inherent triggered abilities of the monarch and the initiative (CR 725.2, 726.2) and of a player with speed (CR
/// 702.179d) that `event` triggers: each the intrinsic ability's index, its controller and what it is about. No object has
/// them, so the rules put them on their way to the stack from an object of their own (`queue`) - but they are triggered
/// abilities like any other (CR 603.2), looked for over the events as `settle::collect_batch` looks for the others, so what
/// stops abilities triggering or makes them trigger again applies to them (CR 603.2d) and they see the history as it was before
/// the event.
pub(crate) fn inherent_triggers(cx: &Cx, event: &Event) -> Vec<(u8, PlayerId, Target)> {
    let mut found = Vec::new();
    let active = cx.s().turn.active;
    match *event {
        // "At the beginning of the monarch's end step, that player draws a card."
        Event::StepBegan { step: Step::End } if cx.s().player(active).monarch && in_game(cx, active) => {
            found.push((MONARCH_DRAW, active, Target::None));
        }
        // "At the beginning of the upkeep of the player who has the initiative, that player ventures into Undercity."
        Event::StepBegan { step: Step::Upkeep } if cx.s().player(active).initiative && in_game(cx, active) => {
            found.push((INITIATIVE_VENTURE, active, Target::None));
        }
        // Taking the initiative ventures into Undercity (CR 726.2), even for one who had it (CR 726.5).
        Event::TookInitiative { player } if in_game(cx, player) => found.push((INITIATIVE_VENTURE, player, Target::None)),
        Event::Damage { source, target: Target::Player(victim), amount, kind, .. } if kind.is_combat() => {
            combat_damage(cx, source, victim, amount, &mut found);
        }
        Event::LifeChange { player, delta } if delta < 0 => {
            if let Some(found_speed) = crate::speed::life_lost(cx, player) {
                found.push(found_speed);
            }
        }
        _ => {}
    }
    found
}

/// Combat damage dealt to the monarch or to the player with the initiative (CR 725.2, 726.2): "whenever a creature
/// deals combat damage to the monarch, its controller becomes the monarch"; "whenever one or more creatures a player
/// controls deal combat damage to the player who has the initiative, the controller of those creatures takes the
/// initiative", once for each such player however many creatures that was.
fn combat_damage(cx: &Cx, source: ObjectId, victim: PlayerId, amount: u32, found: &mut Vec<(u8, PlayerId, Target)>) {
    let player = *cx.s().player(victim);
    if amount == 0 || !(player.monarch || player.initiative) {
        return;
    }
    // A creature, as it last existed (CR 603.10): it may have died dealing the damage.
    let creature = if cx.s().obj(source).zone.is(ZoneKind::Battlefield) {
        crate::layers::chars(cx.cards(), cx.s(), &cx.g.log, source).is(CardType::Creature)
    } else {
        cx.s().last_known(source).is_some_and(|l| l.types.has(CardType::Creature))
    };
    if !creature {
        return;
    }
    let controller = cx.s().obj(source).controller;
    let subject = Target::Player(controller);
    if player.monarch {
        found.push((MONARCH_TAKEN, victim, subject));
    }
    // Once for each player whose creatures dealt it, however many (CR 603.2c): `settle::collect_batch` keeps one per subject.
    if player.initiative {
        found.push((INITIATIVE_TAKEN, victim, subject));
    }
}

/// A player takes the initiative (CR 726.1): whoever had it stops (CR 726.3), and the taker ventures into Undercity
/// (CR 726.2), even one who already had it (CR 726.5).
pub fn take_initiative(cx: &mut Cx, player: PlayerId) {
    if !in_game(cx, player) {
        return;
    }
    for p in cx.s().seats().collect::<Vec<_>>() {
        let is = p == player;
        cx.st().update_player(p, |pl| pl.initiative = is);
    }
    // What watches a player taking it (their venture, CR 726.2) sees it.
    cx.emit(Event::TookInitiative { player });
}

/// Venturing into the Undercity (CR 701.49): enter it at its first room; or from a room move on along an arrow, the
/// player's choice where there are two (CR 309.5a); or from the bottommost room complete it (CR 309.7) and enter it
/// again (CR 701.49c). The room moved into triggers its ability (CR 309.4c).
pub fn venture(cx: &mut Cx, player: PlayerId) -> R<()> {
    let Some(dungeon) = cx.cards().undercity else { return Ok(()) };
    if !in_game(cx, player) {
        return Ok(());
    }
    let def = cx.card(dungeon);
    let here = cx.s().player(player).room;
    let mut room = 0u8;
    if here != 0 {
        let arrows = def.dungeon[here as usize - 1];
        match arrows.len() {
            0 => {
                // Out of the bottommost room: completed, and entered again from the top.
                cx.record(Event::DungeonCompleted { player });
            }
            1 => room = arrows[0],
            _ => {
                let source = cx.resolving().map_or(ObjectId(0), |i| i.object);
                let modes: Vec<Label> = arrows.iter().map(|&r| Label::Card(dungeon, r)).collect();
                let eligible: Vec<u8> = (0..arrows.len() as u8).collect();
                let ask = RequestKind::ChooseModes { source, modes, eligible, count: 1, at_most: 0, affordable: Vec::new(), additional_cost: false, repeat: false };
                let chosen = match cx.ask_core(player, ask, false)? {
                    Answer::Modes(m) => m.first().copied().unwrap_or(0) as usize,
                    _ => 0,
                };
                room = arrows[chosen.min(arrows.len() - 1)];
            }
        }
    }
    cx.st().update_player(player, |p| p.room = room + 1);
    queue(cx, dungeon, room, player, Target::None);
    Ok(())
}

/// State-based action (CR 704.5t): a player whose venture marker is on the bottommost room of a dungeon, with none of
/// that room's abilities on the stack, completes it (CR 309.7). Whether anyone did.
pub fn complete_dungeons(cx: &mut Cx) -> bool {
    let Some(dungeon) = cx.cards().undercity else { return false };
    let def = cx.card(dungeon);
    let mut any = false;
    for p in cx.s().seats().collect::<Vec<_>>() {
        let room = cx.s().player(p).room;
        if room == 0 || !def.dungeon[room as usize - 1].is_empty() {
            continue;
        }
        if cx.s().stack.iter().any(|i| i.kind == StackKind::Triggered && i.ability.card == dungeon && i.controller == p) {
            continue;
        }
        cx.st().update_player(p, |pl| pl.room = 0);
        cx.record(Event::DungeonCompleted { player: p });
        any = true;
    }
    any
}

/// The designations a static keyword ability gives "any time" the board allows it, for the rest of the game: the city's
/// blessing (ascend on a permanent, CR 702.131b: ten or more permanents) and an enduring story (storied, CR 702.195a:
/// three or more permanents that are artifacts, Sagas and/or legendary, each counted once however many of those it is).
/// A player gets one only while they control a permanent with the keyword that has not lost its abilities, and keeps it
/// whatever they control afterwards (CR 702.131c, 702.195b). Checked with the state-based actions, and as each event
/// happens before the abilities it triggers are looked for, so continuous effects that read the designation apply to
/// that look (CR 702.131d, 702.195c). The board is counted only for a player a permanent with one of the keywords works
/// for and who lacks its designation. A face-down permanent (CR 708.2) is the face-down values' copy, so it has neither
/// keyword and, a nameless 2/2 with no supertypes, counts toward a story only if an effect makes it an artifact.
/// Whether anyone got one.
pub fn keyword_designations(cx: &mut Cx) -> bool {
    // Bit `p` of each: player `p` controls a permanent with the keyword and does not have its designation.
    let (mut blessing, mut story) = (0u32, 0u32);
    for &id in cx.s().battlefield() {
        let obj = cx.s().obj(id);
        if !cx.cards().texts(cx.s(), obj).any(|c| cx.cards().designators[c.0 as usize]) {
            continue;
        }
        // A merged permanent has its other components' keywords too (CR 702.140e).
        let (ascend, storied) = cx.cards().texts(cx.s(), obj).fold((false, false), |(a, st), c| (a || cx.card(c).ascend, st || cx.card(c).storied));
        let p = obj.controller;
        let player = cx.s().player(p);
        if player.lost || cx.lost_all_abilities(id) {
            continue;
        }
        if ascend && !player.citys_blessing {
            blessing |= 1 << p.index();
        }
        if storied && !player.enduring_story {
            story |= 1 << p.index();
        }
    }
    if blessing | story == 0 {
        return false;
    }
    let mut got = false;
    for p in cx.s().seats().collect::<Vec<_>>() {
        let bit = 1u32 << p.index();
        if (blessing | story) & bit == 0 {
            continue;
        }
        let mine: Vec<ObjectId> = cx.s().battlefield().iter().copied().filter(|&id| cx.s().obj(id).controller == p).collect();
        if blessing & bit != 0 && mine.len() >= 10 {
            cx.st().update_player(p, |pl| pl.citys_blessing = true);
            got = true;
        }
        if story & bit != 0 {
            let storied = mine
                .iter()
                .filter(|&&id| {
                    let chars = cx.chars(id);
                    chars.is(CardType::Artifact) || chars.has_subtype(Subtype::Saga) || chars.supertypes.legendary
                })
                .take(3)
                .count();
            if storied >= 3 {
                cx.st().update_player(p, |pl| pl.enduring_story = true);
                got = true;
            }
        }
    }
    got
}
