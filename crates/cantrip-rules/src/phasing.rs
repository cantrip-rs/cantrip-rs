//! Phasing (CR 702.26): a permanent's phased-in or phased-out status (CR 110.5). A phased-out permanent is treated as though
//! it does not exist (CR 702.26b), so it is kept off the battlefield's list - which everything that looks at the battlefield
//! reads - in `Zone::PHASED_OUT`, as the same object it was (CR 702.26d): its counters, status, damage, attachments and
//! the effects that apply to it stay with it, a token goes on existing, and neither phasing out nor phasing in is a zone
//! change, so no "leaves" or "enters" ability sees it, and no "becomes attached" or "unattached" one (CR 702.26j).

use cantrip_api::*;
use cantrip_state::Phased;

use crate::Cx;
use crate::layers::live;

/// Whether `id` is still on the battlefield as the object it was, phased in or out: phasing out is no leaving
/// (CR 702.26d), so "until [it] leaves the battlefield" (CR 610.3, 610.4) goes on while it is phased out.
pub fn still_on_battlefield(s: &GameState, id: ObjectId) -> bool {
    live(s, id) && (s.obj(id).zone.is(ZoneKind::Battlefield) || s.obj(id).zone.is(ZoneKind::PhasedOut))
}

/// The permanents attached to any of `hosts`, and those attached to them, found in `list`.
fn attached_to_any(s: &GameState, list: &[ObjectId], hosts: &[ObjectId], only: impl Fn(&Obj) -> bool) -> Vec<ObjectId> {
    let mut found: Vec<ObjectId> = Vec::new();
    let mut frontier: Vec<ObjectId> = hosts.to_vec();
    while let Some(host) = frontier.pop() {
        for &id in list {
            let obj = s.obj(id);
            if obj.attached_to == Some(host) && only(obj) && !found.contains(&id) && !hosts.contains(&id) {
                found.push(id);
                frontier.push(id);
            }
        }
    }
    found
}

/// Phases `permanents` out, all at once (CR 702.26b): each phased-in permanent among them that is still on the battlefield,
/// and with it, indirectly, every Aura, Equipment and Fortification attached to it, and those attached to them (CR
/// 702.26g); one that would phase out both ways phases out only indirectly (CR 702.26h). Each is removed from combat (CR
/// 506.4). `until_leaves` is "phases out until [that permanent] leaves the battlefield" (CR 610.4): nothing phases out if
/// it has already left (CR 610.4b; Oubliette's ruling), and what does phases in only once it has (`phase_in_returning`),
/// tapped if `tapped_as_it_returns`.
pub fn phase_out(cx: &mut Cx, permanents: &[ObjectId], until_leaves: Option<ObjectId>, tapped_as_it_returns: bool) {
    if until_leaves.is_some_and(|by| !still_on_battlefield(cx.s(), by)) {
        return;
    }
    let mut direct: Vec<ObjectId> = Vec::new();
    for &id in permanents {
        if live(cx.s(), id) && cx.s().obj(id).zone.is(ZoneKind::Battlefield) && !direct.contains(&id) {
            direct.push(id);
        }
    }
    if direct.is_empty() {
        return;
    }
    let indirect = attached_to_any(cx.s(), cx.s().battlefield(), &direct, |_| true);
    direct.retain(|id| !indirect.contains(id));
    let all: Vec<(ObjectId, bool)> = direct.iter().map(|&id| (id, false)).chain(indirect.iter().map(|&id| (id, true))).collect();
    // CR 608.2h, 702.26b,d,g: every source's information is captured against the same phased-in board, attachments
    // and effects included, before combat removal or a duration ending changes that board.
    let snapshots: Vec<_> = all.iter().map(|&(id, indirect)| (id, indirect, crate::ops::last_known_of(cx, id))).collect();
    let leaving = |id: ObjectId| all.iter().any(|&(a, _)| a == id);
    if cx.s().attackers.iter().any(|a| leaving(a.id)) {
        let attackers = cx.s().attackers.iter().copied().filter(|a| !leaving(a.id)).collect();
        cx.st().set_attackers(attackers);
    }
    if cx.s().blocks.iter().any(|b| leaving(b.blocker) || leaving(b.attacker)) {
        let blocks = cx.s().blocks.iter().copied().filter(|b| !leaving(b.blocker) && !leaving(b.attacker)).collect();
        cx.st().set_blocks(blocks);
    }
    // CR 702.26f: a "for as long as" duration that tracks one of them ends, as it can no longer see it (Teferi's Protection's
    // ruling on Mathas, Fiend Seeker): one about its counters or its being tapped. (One about an effect's source on the
    // battlefield ends with the state-based check, `WhileSourceOnBattlefield`.)
    let tracks = |e: &ContinuousEffect| leaving(e.affected) && matches!(e.duration, Duration::WhileCounter(_) | Duration::WhileTapped);
    if cx.s().effects.iter().any(tracks) {
        let effects_before = cx.s().effects.len();
        cx.st().effects_retain(|e| !tracks(e));
        // An effect that ends changes the game with no event (CR 603.8).
        if cx.s().effects.len() < effects_before {
            cx.state_changed();
        }
    }
    for (id, indirect, snapshot) in snapshots {
        let controller = cx.s().obj(id).controller;
        let last_known = cx.st().last_known_push(snapshot);
        let phased = if indirect {
            Phased { last_known, under: controller, indirect, until_leaves: None, tapped_as_it_returns: false }
        } else {
            Phased { last_known, under: controller, indirect, until_leaves, tapped_as_it_returns }
        };
        cx.st().zone_remove(Zone::BATTLEFIELD, id);
        cx.st().zone_push(Zone::PHASED_OUT, id);
        cx.st().update_obj(id, |o| {
            o.zone = Zone::PHASED_OUT;
            o.marks.phased = Some(phased);
        });
        cx.record(Event::PhasedOut { permanent: id, controller, indirect });
    }
    // CR 702.26f: so does a "for as long as you control" permission about one of them, for good though it phase in.
    if cx.s().permissions.iter().any(|p| p.grant.expiry.controlled().is_some_and(leaving)) {
        cx.st().end_lapsed_permissions();
    }
}

/// Phases in, all at once (CR 702.26c), each of `permanents` that phased out directly, and with each everything that phased
/// out indirectly with it (CR 702.26g), attached as it was. An Aura, Equipment or Fortification that phased out directly
/// phases in attached to what it was attached to only if that is still on the battlefield, or that player still in the
/// game; otherwise it phases in unattached, for the state-based actions to deal with (CR 702.26i, 704.5m, 704.5n). A
/// permanent goes back to whoever its control-changing effects now say controls it (CR 613.1b: the rulings on one that
/// expired while it was phased out).
fn phase_in(cx: &mut Cx, permanents: &[ObjectId]) {
    let direct: Vec<ObjectId> = permanents
        .iter()
        .copied()
        .filter(|&id| cx.s().obj(id).marks.phased.is_some_and(|p| !p.indirect) && cx.s().obj(id).zone.is(ZoneKind::PhasedOut))
        .collect();
    if direct.is_empty() {
        return;
    }
    let list = cx.s().zone(Zone::PHASED_OUT).to_vec();
    let indirect = attached_to_any(cx.s(), &list, &direct, |o| o.marks.phased.is_some_and(|p| p.indirect));
    let all: Vec<ObjectId> = direct.iter().chain(indirect.iter()).copied().collect();
    for &id in &all {
        let tapped = cx.s().obj(id).marks.phased.is_some_and(|p| p.tapped_as_it_returns);
        cx.st().zone_remove(Zone::PHASED_OUT, id);
        cx.st().zone_push(Zone::BATTLEFIELD, id);
        cx.st().update_obj(id, |o| {
            o.zone = Zone::BATTLEFIELD;
            o.marks.phased = None;
            // Oubliette's "tap that creature as it phases in this way": it is tapped as it phases in, which is no
            // becoming tapped that an ability sees (the ruling).
            o.tapped |= tapped;
        });
    }
    for &id in &direct {
        let obj = *cx.s().obj(id);
        if let Some(host) = obj.attached_to
            && !(live(cx.s(), host) && cx.s().obj(host).zone.is(ZoneKind::Battlefield))
        {
            cx.st().update_obj(id, |o| o.attached_to = None);
        }
        if let Some(p) = obj.attached_player
            && !cx.s().in_game(p)
        {
            cx.st().update_obj(id, |o| o.attached_player = None);
        }
    }
    // Control is settled for every permanent: one that phased in may decide another's controller (an Aura that gives
    // control phasing in on its own, CR 702.26).
    crate::ops::settle_control(cx);
    for &id in &all {
        // CR 722.3c: one that phases in prepared has its prepare spell's copy made in exile again, its old one having
        // ceased to exist while it was phased out (no becoming prepared: nothing that watches that sees it) - unless the
        // state-based actions have not yet ended the old one, which then stays the one copy.
        let has_copy = |s: &GameState| {
            s.seats().any(|p| s.zone(Zone::exile(p)).iter().any(|&c| s.obj(c).card_copy && s.obj(c).prepared_from == Some(id)))
        };
        if cx.s().obj(id).prepared && !has_copy(cx.s()) {
            crate::ops::prepared_copy(cx, id);
        }
        cx.record(Event::PhasedIn { permanent: id });
    }
}

/// The untap step's first turn-based action (CR 502.1, 703.4a): every permanent that phased out directly under the active
/// player's control phases in, but not one phased out "until" an event (CR 610.4a). One that phased out under a player who
/// has since left the game phases in during the next untap step after that player's next turn would have begun (CR
/// 702.26n): the untap step of the first player still in the game after them in turn order, unless the turn just ended
/// was their own (they left during it, and their next turn is a round away). (Permanents with the phasing keyword would
/// phase out at the same time; no card here has it.)
pub fn untap_step(cx: &mut Cx, active: PlayerId) {
    let s = cx.s();
    if s.zone(Zone::PHASED_OUT).is_empty() {
        return;
    }
    let would_have_begun = |left: PlayerId| {
        !s.in_game(left)
            && s.turn_order_from(s.next_player(left)).next() == Some(active)
            && s.turn.number.saturating_sub(1) != s.player(left).turn_began
    };
    let due: Vec<ObjectId> = s
        .zone(Zone::PHASED_OUT)
        .iter()
        .copied()
        .filter(|&id| {
            s.obj(id).marks.phased.is_some_and(|p| !p.indirect && p.until_leaves.is_none() && (p.under == active || would_have_begun(p.under)))
        })
        .collect();
    phase_in(cx, &due);
}

/// CR 610.4: a permanent phased out "until [a permanent] leaves the battlefield" phases in, as a one-shot effect that
/// follows the event, once that permanent has left; Oubliette's ruling: "immediately after Oubliette leaves the
/// battlefield". Whether any did.
pub fn phase_in_returning(cx: &mut Cx) -> bool {
    let s = cx.s();
    if s.zone(Zone::PHASED_OUT).is_empty() {
        return false;
    }
    let due: Vec<ObjectId> = s
        .zone(Zone::PHASED_OUT)
        .iter()
        .copied()
        .filter(|&id| s.obj(id).marks.phased.and_then(|p| p.until_leaves).is_some_and(|by| !still_on_battlefield(s, by)))
        .collect();
    if due.is_empty() {
        return false;
    }
    phase_in(cx, &due);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Game;

    static BEAR: CardDef = CardDef {
        name: "Phasing Test Bear",
        types: TypeSet::of(&[CardType::Creature]),
        power: Some(2),
        toughness: Some(2),
        ..CardDef::BLANK
    };
    static AURA: CardDef = CardDef { name: "Phasing Test Aura", types: TypeSet::of(&[CardType::Enchantment]), ..CardDef::BLANK };
    static WARDEN: CardDef = CardDef { name: "Phasing Test Warden", types: TypeSet::of(&[CardType::Enchantment]), ..CardDef::BLANK };

    /// A game with player 0's Bear, an Aura of player 1's attached to it, and player 0's Warden, all on the battlefield.
    fn game() -> (Game, ObjectId, ObjectId, ObjectId) {
        let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAR, &AURA, &WARDEN])));
        let mut game = Game::new(table, &crate::Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
        let put = |game: &mut Game, def: &'static CardDef, p: u8| {
            let id = game.state.push_obj(Obj::new(table.id_of(def), PlayerId(p), Zone::BATTLEFIELD));
            game.state.zone_push(Zone::BATTLEFIELD, id);
            id
        };
        let bear = put(&mut game, &BEAR, 0);
        let aura = put(&mut game, &AURA, 1);
        let warden = put(&mut game, &WARDEN, 0);
        game.state.update_obj(aura, |o| o.attached_to = Some(bear));
        (game, bear, aura, warden)
    }

    fn cx(game: &mut Game) -> Cx<'_> {
        Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None }
    }

    #[test]
    fn a_permanent_phases_out_with_its_aura_and_both_phase_in_in_its_controllers_untap_step() {
        let (mut game, bear, aura, warden) = game();
        let mut cx = cx(&mut game);
        phase_out(&mut cx, &[bear], None, false);
        let s = cx.s();
        assert_eq!(s.battlefield(), &[warden]);
        assert_eq!(s.zone(Zone::PHASED_OUT), &[bear, aura]);
        // The same objects, the Aura still attached (CR 702.26d, 702.26g); it phased out indirectly, under its own controller.
        assert!(live(s, bear) && live(s, aura));
        assert_eq!(s.obj(aura).attached_to, Some(bear));
        assert_eq!(s.obj(aura).marks.phased.map(|p| (p.indirect, p.under)), Some((true, PlayerId(1))));
        assert_eq!(s.obj(bear).marks.phased.map(|p| (p.indirect, p.under)), Some((false, PlayerId(0))));
        // Not the Bear's controller's untap step: nothing phases in, the indirect Aura's controller's no more than another's.
        untap_step(&mut cx, PlayerId(1));
        assert_eq!(cx.s().zone(Zone::PHASED_OUT).len(), 2);
        untap_step(&mut cx, PlayerId(0));
        let s = cx.s();
        assert!(s.zone(Zone::PHASED_OUT).is_empty());
        assert_eq!(s.battlefield(), &[warden, bear, aura]);
        assert_eq!(s.obj(aura).attached_to, Some(bear));
        assert!(s.obj(bear).marks.phased.is_none() && s.obj(aura).marks.phased.is_none());
        assert!(s.obj(bear).zone.is(ZoneKind::Battlefield));
    }

    #[test]
    fn a_permanent_phased_out_until_another_leaves_phases_in_tapped_only_once_that_one_has_left() {
        let (mut game, bear, aura, warden) = game();
        let mut cx = cx(&mut game);
        phase_out(&mut cx, &[bear], Some(warden), true);
        // CR 610.4a: not in an untap step.
        untap_step(&mut cx, PlayerId(0));
        assert!(!phase_in_returning(&mut cx));
        assert_eq!(cx.s().zone(Zone::PHASED_OUT), &[bear, aura]);
        // The Warden phasing out is no leaving (CR 702.26d).
        phase_out(&mut cx, &[warden], None, false);
        assert!(!phase_in_returning(&mut cx));
        untap_step(&mut cx, PlayerId(0));
        crate::ops::move_to(&mut cx, warden, Zone::graveyard(PlayerId(0)));
        assert!(phase_in_returning(&mut cx));
        let s = cx.s();
        assert_eq!(s.battlefield(), &[bear, aura]);
        assert!(s.obj(bear).tapped && !s.obj(aura).tapped);
    }

    #[test]
    fn a_phased_out_permanent_is_moved_by_nothing() {
        let (mut game, bear, aura, _) = game();
        let mut cx = cx(&mut game);
        phase_out(&mut cx, &[bear], None, false);
        // CR 702.26b: "Return it to your hand" from an ability that remembered it does nothing.
        assert!(crate::ops::return_to_hand(&mut cx, &[bear]).is_empty());
        assert!(crate::ops::exile_all(&mut cx, &[aura]).is_empty());
        assert_eq!(cx.s().zone(Zone::PHASED_OUT), &[bear, aura]);
        assert!(live(cx.s(), bear) && live(cx.s(), aura));
    }

    #[test]
    fn nothing_phases_out_until_a_permanent_that_has_already_left() {
        let (mut game, bear, _, warden) = game();
        let mut cx = cx(&mut game);
        crate::ops::move_to(&mut cx, warden, Zone::graveyard(PlayerId(0)));
        phase_out(&mut cx, &[bear], Some(warden), true);
        assert!(cx.s().zone(Zone::PHASED_OUT).is_empty());
        assert!(!cx.s().obj(bear).tapped);
    }

    #[test]
    fn an_aura_that_phased_out_alone_phases_in_unattached_once_its_creature_is_gone() {
        let (mut game, bear, aura, _) = game();
        let mut cx = cx(&mut game);
        phase_out(&mut cx, &[aura], None, false);
        assert_eq!(cx.s().obj(aura).marks.phased.map(|p| p.indirect), Some(false));
        crate::ops::move_to(&mut cx, bear, Zone::graveyard(PlayerId(0)));
        untap_step(&mut cx, PlayerId(1));
        let s = cx.s();
        assert!(s.obj(aura).zone.is(ZoneKind::Battlefield));
        assert_eq!(s.obj(aura).attached_to, None);
    }

    /// A game of three: player 0's turn `number`, player 2 (who leaves) last began a turn on `began`. Player 2 controls two
    /// of player 0's Bears, one by a control-changing effect and one with none, and both have phased out under player 2.
    fn three_players(number: u32, active: PlayerId, began: u32) -> (Game, ObjectId, ObjectId) {
        let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAR])));
        let mut game = Game::new(table, &crate::Setup { decks: vec![Vec::new(), Vec::new(), Vec::new()], ..Default::default() });
        let put = |game: &mut Game| {
            let id = game.state.push_obj(Obj::new(table.id_of(&BEAR), PlayerId(0), Zone::BATTLEFIELD));
            game.state.zone_push(Zone::BATTLEFIELD, id);
            id
        };
        let stolen = put(&mut game);
        let kept = put(&mut game);
        game.state.update_obj(kept, |o| o.controller = PlayerId(2));
        game.state.update_turn(|t| {
            t.number = number;
            t.active = active;
        });
        game.state.update_player(PlayerId(2), |p| p.turn_began = began);
        let mut cx = cx(&mut game);
        cantrip_api::Game::add_effect(&mut cx, stolen, stolen, EffectKind::Control(PlayerId(2)), Duration::Indefinite);
        assert_eq!(cx.s().obj(stolen).controller, PlayerId(2));
        phase_out(&mut cx, &[stolen, kept], None, false);
        (game, stolen, kept)
    }

    fn untap_step_of(game: &mut Game, number: u32, active: PlayerId) {
        game.state.update_turn(|t| {
            t.number = number;
            t.active = active;
        });
        untap_step(&mut cx(game), active);
    }

    /// CR 800.4a, 702.26n: player 2 leaves during player 1's turn 5. The Bear they controlled with no effect is exiled though
    /// phased out; the one an effect gave them goes back to player 0 and phases in during the next untap step after player
    /// 2's next turn would have begun: player 0's, turn 7.
    #[test]
    fn a_permanent_phased_out_under_a_player_who_left_phases_in_after_their_next_turn_would_have_begun() {
        let (mut game, stolen, kept) = three_players(5, PlayerId(1), 3);
        game.state.update_player(PlayerId(2), |p| p.lost = true);
        crate::ops::leave_game(&mut cx(&mut game), PlayerId(2));
        assert!(game.state.zone(Zone::exile(PlayerId(0))).len() == 1 && !game.state.zone(Zone::PHASED_OUT).contains(&kept));
        assert_eq!(game.state.zone(Zone::PHASED_OUT), &[stolen]);
        assert_eq!(game.state.obj(stolen).controller, PlayerId(0));
        untap_step_of(&mut game, 6, PlayerId(1));
        assert_eq!(game.state.zone(Zone::PHASED_OUT), &[stolen]);
        untap_step_of(&mut game, 7, PlayerId(0));
        assert!(game.state.zone(Zone::PHASED_OUT).is_empty());
        assert!(game.state.battlefield().contains(&stolen));
        assert_eq!(game.state.obj(stolen).controller, PlayerId(0));
    }

    /// Player 2 leaves during their own turn 6: their next turn would have begun a round later, so the Bear is still phased
    /// out after player 0's untap step on turn 7 and phases in on turn 10.
    #[test]
    fn a_player_who_left_during_their_own_turn_has_their_next_turn_a_round_later() {
        let (mut game, stolen, _) = three_players(6, PlayerId(2), 6);
        game.state.update_player(PlayerId(2), |p| p.lost = true);
        crate::ops::leave_game(&mut cx(&mut game), PlayerId(2));
        untap_step_of(&mut game, 7, PlayerId(0));
        assert_eq!(game.state.zone(Zone::PHASED_OUT), &[stolen]);
        untap_step_of(&mut game, 10, PlayerId(0));
        assert!(game.state.zone(Zone::PHASED_OUT).is_empty());
    }

    static JUMP: CardDef = CardDef { name: "Phasing Test Jump", types: TypeSet::of(&[CardType::Instant]), ..CardDef::BLANK };
    static AVIATOR: CardDef = CardDef { prepare: Some(&JUMP), name: "Phasing Test Aviator", ..BEAR };

    /// CR 722.3c: a prepared permanent phasing in makes its prepare spell's copy again - once: if the state-based actions
    /// have not yet ended the copy it had (phased out and in again before they were checked), that one stays the only one.
    #[test]
    fn a_prepared_permanent_phasing_in_has_one_copy_of_its_prepare_spell() {
        let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&AVIATOR, &JUMP])));
        let mut game = Game::new(table, &crate::Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
        let aviator = game.state.push_obj(Obj::new(table.id_of(&AVIATOR), PlayerId(0), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, aviator);
        let mut cx = cx(&mut game);
        crate::ops::become_prepared(&mut cx, aviator);
        let copies = |cx: &Cx| cx.s().zone(Zone::exile(PlayerId(0))).iter().filter(|&&c| cx.s().obj(c).prepared_from == Some(aviator)).count();
        assert_eq!(copies(&cx), 1);
        phase_out(&mut cx, &[aviator], None, false);
        untap_step(&mut cx, PlayerId(0));
        assert!(cx.s().battlefield().contains(&aviator) && cx.s().obj(aviator).prepared);
        assert_eq!(copies(&cx), 1);
    }

    fn creature(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool { c.is(CardType::Creature) }
    fn lifelink(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) { c.keywords = c.keywords.with(Keyword::Lifelink); }
    static LORD: CardDef = CardDef {
        name: "Phasing Source Test Lord", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2),
        statics: &[static_ability(Layer::Ability, creature, lifelink, "Creatures have lifelink.")], ..CardDef::BLANK
    };

    #[test]
    fn simultaneous_phase_out_keeps_granted_source_information_not_projected_values() {
        let table = Box::leak(Box::new(CardTable::new([&BEAR, &LORD])));
        let mut game = Game::new(table, &crate::Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
        let put = |g: &mut Game, def| {
            let id = g.state.push_obj(Obj::new(table.id_of(def), PlayerId(0), Zone::BATTLEFIELD));
            g.state.zone_push(Zone::BATTLEFIELD, id); g.state.see_statics(); id
        };
        // Lord goes first to prove snapshots are not taken after earlier permanents already phased out.
        let lord = put(&mut game, &LORD);
        let bear = put(&mut game, &BEAR);
        let mut context = cx(&mut game);
        crate::counters::put(&mut context, bear, CounterKind::PlusOne, 2, PlayerId(0), crate::counters::How::Effect);
        phase_out(&mut context, &[lord, bear], None, false);
        assert!(Query::chars(&context, bear).has(Keyword::Lifelink));
        assert!(!crate::layers::chars(context.cards(), context.s(), &context.g.log, bear).has(Keyword::Lifelink), "projected values were permanently frozen");
        assert_eq!(Query::chars(&context, bear).power, Some(4));
        assert_eq!(crate::ops::damage(&mut context, bear, Target::Player(PlayerId(1)), 2, false), 2);
        assert_eq!(context.s().player(PlayerId(0)).life, 22);
        phase_in(&mut context, &[lord, bear]);
        assert!(context.s().phased_last_known(bear).is_none());
        assert!(Query::chars(&context, bear).has(Keyword::Lifelink));
        assert_eq!(context.s().obj(bear).counters(CounterKind::PlusOne), 2);
    }

    #[test]
    fn phase_in_recomputes_expired_effects_and_second_phase_out_refreshes_snapshot() {
        let (mut game, bear, _, _) = game();
        let mut context = cx(&mut game);
        cantrip_api::Game::gain_control(&mut context, bear, bear, PlayerId(1), Duration::EndOfTurn);
        cantrip_api::Game::add_effect(&mut context, bear, bear, EffectKind::AddKeywords(Keywords::of(&[Keyword::Lifelink])), Duration::EndOfTurn);
        cantrip_api::Game::add_effect(&mut context, bear, bear, EffectKind::ChangeText { from: Word::Subtype(Subtype::Forest), to: Word::Subtype(Subtype::Island) }, Duration::EndOfTurn);
        phase_out(&mut context, &[bear], None, false);
        let first = context.s().obj(bear).marks.phased.unwrap().last_known;
        context.st().effects_retain(|e| e.duration != Duration::EndOfTurn);
        assert_eq!(Query::controller(&context, bear), PlayerId(1));
        assert!(Query::chars(&context, bear).has(Keyword::Lifelink));
        assert_eq!(Query::words(&context, bear).subtype(Subtype::Forest), Subtype::Island);
        assert_eq!(crate::ops::damage(&mut context, bear, Target::Player(PlayerId(0)), 1, false), 1);
        assert_eq!(context.s().player(PlayerId(1)).life, 21, "expired control failed to use phased-in controller snapshot");
        phase_in(&mut context, &[bear]);
        assert_eq!(Query::controller(&context, bear), PlayerId(0));
        assert!(!Query::chars(&context, bear).has(Keyword::Lifelink));
        assert_eq!(Query::words(&context, bear).subtype(Subtype::Forest), Subtype::Forest);
        phase_out(&mut context, &[bear], None, false);
        assert_ne!(context.s().obj(bear).marks.phased.unwrap().last_known, first);
        assert!(!Query::chars(&context, bear).has(Keyword::Lifelink), "second snapshot reused first phase-out");
    }

    #[test]
    fn ability_loss_before_phase_out_is_in_snapshot_and_protection_checks_use_its_quality() {
        let (mut game, bear, _, _) = game();
        game.state.update_turn(|t| t.number = 1);
        let warden = game.state.push_obj(Obj::new(game.cards.id_of(&BEAR), PlayerId(1), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, warden);
        let mut context = cx(&mut game);
        cantrip_api::Game::add_effect(&mut context, bear, bear, EffectKind::AddKeywords(Keywords::of(&[Keyword::Lifelink])), Duration::EndOfTurn);
        cantrip_api::Game::add_effect(&mut context, bear, bear, EffectKind::LoseAllAbilities, Duration::EndOfTurn);
        cantrip_api::Game::add_effect(&mut context, bear, bear, EffectKind::SetColors(ColorSet::of(Color::Red)), Duration::EndOfTurn);
        cantrip_api::Game::add_effect(&mut context, bear, warden, EffectKind::Protection(ColorSet::of(Color::Red)), Duration::Indefinite);
        phase_out(&mut context, &[bear], None, false);
        context.st().effects_retain(|e| e.duration != Duration::EndOfTurn);
        assert!(!Query::chars(&context, bear).has(Keyword::Lifelink));
        assert_eq!(Query::chars(&context, bear).colors, ColorSet::of(Color::Red));
        assert!(crate::layers::chars(context.cards(), context.s(), &context.g.log, warden).protection.has(Color::Red));
        assert_eq!(crate::ops::protection_prevents(&context, bear, Target::Object(warden), false), Some(warden));
        assert!(!crate::layers::can_target(context.cards(), context.s(), &context.g.log, &Asker { source: bear, controller: PlayerId(0) }, Target::Object(warden)));
        assert_eq!(crate::ops::damage(&mut context, bear, Target::Object(warden), 1, false), 0);
        assert_eq!(context.s().obj(warden).damage, 0);
        phase_in(&mut context, &[bear]);
        assert_eq!(Query::chars(&context, bear).colors, ColorSet::NONE);
    }

    #[test]
    fn phasing_snapshot_and_queries_round_trip_through_undo_redo() {
        let (mut game, bear, _, _) = game();
        let mut context = cx(&mut game);
        cantrip_api::Game::add_effect(&mut context, bear, bear, EffectKind::AddKeywords(Keywords::of(&[Keyword::Lifelink])), Duration::EndOfTurn);
        let before = context.s().clone_with_journal();
        let mark = context.s().mark();
        phase_out(&mut context, &[bear], None, false);
        let after = context.s().clone_with_journal();
        let (undos, redos) = context.st().step_back(mark);
        assert!(context.s() == &before);
        assert!(context.s().phased_last_known(bear).is_none());
        context.st().step_forward(undos, redos);
        assert!(context.s() == &after);
        assert!(Query::chars(&context, bear).has(Keyword::Lifelink));
    }
}
