//! The continuous world-supertype interval (CR 205.4f, 704.5k): Arboria and Concordant Crossroads.
//! This is present rules state, like an effect's timestamp, rather than a remembered count of past events.
use crate::{Cx, live};
use cantrip_api::*;

/// Synchronize the current world intervals after one whole instruction, or an otherwise ungrouped change of
/// characteristics. New intervals share one clock value; remaining a World does not refresh that value.
/// Phased-out permanents preserve their interval (CR 702.26), and are not considered by the world rule.
pub fn sync(cx: &mut Cx) {
    // Only where a World could be involved: a permanent (or emblem) with a World interval, one whose text is a World
    // card's, or one with a static ability that can make something a World (`StaticDef::makes_world`). Guarded on the
    // objects of this game, never on the card registry, and read as asked rather than kept: it is asked after every
    // event, where a pass over every static source would cost the games with no World (`library_visibility::sync` guards
    // the same way).
    let s = cx.s();
    let possible = s.battlefield().iter().chain(s.zone(Zone::COMMAND)).any(|&id| {
        let obj = s.obj(id);
        obj.world_since != 0 || cx.cards().texts(s, obj).any(|card| cx.cards().world_sensitive[card.0 as usize])
    });
    if !possible {
        return;
    }
    let changes: Vec<_> = cx
        .s()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| live(cx.s(), id) && cx.s().obj(id).marks.phased.is_none())
        .filter_map(|id| {
            let world = cx.chars(id).supertypes.world;
            (world != (cx.s().obj(id).world_since != 0)).then_some((id, world))
        })
        .collect();
    let since = if changes.iter().any(|&(_, world)| world) {
        cx.st().timestamp()
    } else {
        0
    };
    for (id, world) in changes {
        cx.st()
            .update_obj(id, |obj| obj.world_since = if world { since } else { 0 });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Game, Setup};
    const A: PlayerId = PlayerId(0);
    const B: PlayerId = PlayerId(1);
    static WORLD: CardDef = CardDef {
        name: "World fixture",
        types: TypeSet::of(&[CardType::Enchantment]),
        supertypes: Supertypes {
            world: true,
            basic: false,
            legendary: false,
            snow: false,
        },
        keywords: Keywords::of(&[Keyword::Indestructible]),
        ..CardDef::BLANK
    };
    static PLAIN: CardDef = CardDef {
        name: "Ordinary fixture",
        supertypes: Supertypes {
            world: false,
            basic: false,
            legendary: false,
            snow: false,
        },
        ..WORLD
    };
    static LAND: CardDef = CardDef {
        name: "World test land",
        types: TypeSet::of(&[CardType::Land]),
        ..CardDef::BLANK
    };
    fn lands(q: &dyn LayerQuery, source: ObjectId, _: ObjectId, c: &Chars) -> bool {
        q.state().obj(source).tapped && c.is(CardType::Land)
    }
    fn make_world(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
        c.supertypes.world = true;
    }
    static SETTER: CardDef = CardDef {
        name: "World type layer fixture",
        statics: &[static_ability(
            Layer::Type,
            lands,
            make_world,
            "Lands are world while this is tapped",
        ).makes_world()],
        ..PLAIN
    };
    fn history_lands(q: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
        c.is(CardType::Land)
            && q.history()
                .all()
                .iter()
                .filter(|r| matches!(r.event, Event::CoinFlipped { .. }))
                .count()
                % 2
                == 1
    }
    static HISTORY: CardDef = CardDef {
        name: "History-dependent world fixture",
        statics: &[static_ability(
            Layer::Type,
            history_lands,
            make_world,
            "Lands are world after an odd number of flips",
        ).makes_world()],
        ..PLAIN
    };
    fn every_land(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
        c.is(CardType::Land)
    }
    /// A static ability that makes Worlds without saying so (`StaticDef::makes_world`).
    static UNMARKED: CardDef = CardDef {
        name: "Unmarked world-making fixture",
        statics: &[static_ability(Layer::Type, every_land, make_world, "Lands are world")],
        ..PLAIN
    };
    fn game() -> Game {
        let cards = Box::leak(Box::new(CardTable::new([
            &WORLD, &PLAIN, &LAND, &SETTER, &HISTORY, &UNMARKED,
        ])));
        Game::new(
            cards,
            &Setup {
                decks: vec![Vec::new(), Vec::new()],
                ..Default::default()
            },
        )
    }
    fn cx(g: &mut Game) -> Cx<'_> {
        Cx {
            g,
            frame: usize::MAX,
            cursor: 0,
            mana_source: None,
            entering: None,
            lineage: Vec::new(),
            damage: None,
            asked_at: None,
            doer: Some(A),
            producing: None,
            reached: None,
        }
    }
    fn place(g: &mut Game, def: &'static CardDef, owner: PlayerId) -> ObjectId {
        let mut object = Obj::new(g.cards.id_of(def), owner, Zone::BATTLEFIELD);
        object.timestamp = g.state.timestamp();
        let id = g.state.push_obj(object);
        g.state.zone_push(Zone::BATTLEFIELD, id);
        g.state.see_statics();
        sync(&mut cx(g));
        id
    }
    fn judge(g: &mut Game) {
        crate::settle::state_based_actions_alone(&mut cx(g))
            .expect("the world rule asks no choice");
    }
    fn remains(g: &Game, id: ObjectId) -> bool {
        g.state.obj(id).became.is_none() && g.state.obj(id).zone.is(ZoneKind::Battlefield)
    }
    #[test]
    fn newest_world_wins_across_controllers_without_destroying_or_sacrificing() {
        let mut g = game();
        let old = place(&mut g, &WORLD, A);
        let new = place(&mut g, &WORLD, B);
        judge(&mut g);
        assert!(!remains(&g, old));
        assert!(remains(&g, new));
        assert!(g.log.iter().any(|r| matches!(r.event,Event::ZoneChange {old:id,to,reason,..} if id==old && to.is(ZoneKind::Graveyard) && reason!=ZoneReason::Destroy && reason!=ZoneReason::Sacrifice)));
    }
    #[test]
    fn simultaneous_new_worlds_tie_and_send_every_world_to_the_graveyard() {
        let mut g = game();
        let old = place(&mut g, &WORLD, A);
        let card = g.cards.id_of(&WORLD);
        let mut hand = Vec::new();
        for owner in [A, B] {
            let id = g.state.push_obj(Obj::new(card, owner, Zone::hand(owner)));
            g.state.zone_push(Zone::hand(owner), id);
            hand.push((id, owner, false));
        }
        let entered =
            cantrip_api::Game::put_all_onto_battlefield_as(&mut cx(&mut g), &hand).unwrap();
        assert_eq!(entered.len(), 2);
        assert_eq!(
            g.state.obj(entered[0]).world_since,
            g.state.obj(entered[1]).world_since
        );
        judge(&mut g);
        for id in [old, entered[0], entered[1]] {
            assert!(!remains(&g, id));
        }
    }
    #[test]
    fn layer_timestamps_do_not_refresh_a_continuous_world_interval_but_losing_world_does() {
        let mut g = game();
        let first = place(&mut g, &WORLD, A);
        let second = place(&mut g, &WORLD, B);
        let since = g.state.obj(first).world_since;
        let timestamp = g.state.timestamp();
        g.state.update_obj(first, |o| o.timestamp = timestamp);
        let world = g.cards.id_of(&WORLD);
        cantrip_api::Game::set_copy(&mut cx(&mut g), first, Copiable::of(world));
        assert_eq!(g.state.obj(first).world_since, since);
        assert!(since < g.state.obj(second).world_since);
        let plain = g.cards.id_of(&PLAIN);
        cantrip_api::Game::set_copy(&mut cx(&mut g), first, Copiable::of(plain));
        assert_eq!(g.state.obj(first).world_since, 0);
        cantrip_api::Game::set_copy(&mut cx(&mut g), first, Copiable::of(world));
        assert!(g.state.obj(first).world_since > g.state.obj(second).world_since);
        judge(&mut g);
        assert!(remains(&g, first));
        assert!(!remains(&g, second));
    }
    #[test]
    fn phasing_preserves_world_age_and_returning_an_older_world_does_not_save_it() {
        let mut g = game();
        let old = place(&mut g, &WORLD, A);
        let since = g.state.obj(old).world_since;
        crate::phasing::phase_out(&mut cx(&mut g), &[old], None, false);
        let new = place(&mut g, &WORLD, B);
        judge(&mut g);
        assert!(remains(&g, new));
        crate::phasing::untap_step(&mut cx(&mut g), A);
        assert_eq!(g.state.obj(old).world_since, since);
        judge(&mut g);
        assert!(!remains(&g, old));
        assert!(remains(&g, new));
    }
    #[test]
    fn a_type_layer_can_make_several_existing_permanents_world_at_once() {
        let mut g = game();
        let older = place(&mut g, &WORLD, A);
        let setter = place(&mut g, &SETTER, A);
        let first = place(&mut g, &LAND, A);
        let second = place(&mut g, &LAND, B);
        cantrip_api::Game::tap(&mut cx(&mut g), setter);
        assert!(g.chars(first).supertypes.world);
        assert_ne!(g.state.obj(first).world_since, 0);
        assert_eq!(
            g.state.obj(first).world_since,
            g.state.obj(second).world_since
        );
        judge(&mut g);
        assert!(!remains(&g, older));
        assert!(!remains(&g, first));
        assert!(!remains(&g, second));
        assert!(remains(&g, setter));
    }
    #[test]
    fn phased_out_worlds_keep_their_interval_until_they_are_visible_again() {
        let mut g = game();
        let setter = place(&mut g, &SETTER, A);
        let land = place(&mut g, &LAND, A);
        cantrip_api::Game::tap(&mut cx(&mut g), setter);
        let since = g.state.obj(land).world_since;
        assert_ne!(since, 0);
        crate::phasing::phase_out(&mut cx(&mut g), &[land], None, false);
        cantrip_api::Game::untap(&mut cx(&mut g), setter);
        assert_eq!(
            g.state.obj(land).world_since,
            since,
            "do not modify a phased-out permanent"
        );
        crate::phasing::untap_step(&mut cx(&mut g), A);
        sync(&mut cx(&mut g));
        assert!(!g.chars(land).supertypes.world);
        assert_eq!(g.state.obj(land).world_since, 0);
    }
    #[test]
    fn history_changes_sync_world_intervals_but_simultaneous_records_do_not_create_false_intervals()
    {
        let mut g = game();
        let older = place(&mut g, &WORLD, A);
        place(&mut g, &HISTORY, A);
        let land = place(&mut g, &LAND, A);
        let flip = Event::CoinFlipped {
            player: A,
            result: CoinFlip::WonHeads,
        };
        cx(&mut g).record_batch(&[flip, flip]);
        assert_eq!(g.state.obj(land).world_since, 0);
        cx(&mut g).record(flip);
        assert!(g.state.obj(land).world_since > g.state.obj(older).world_since);
        cx(&mut g).record(flip);
        assert_eq!(g.state.obj(land).world_since, 0);
        judge(&mut g);
        assert!(remains(&g, older));
        assert!(remains(&g, land));
    }
    #[test]
    fn world_interval_order_is_journalled_and_normalized_for_loop_identity() {
        let mut g = game();
        let first = place(&mut g, &WORLD, A);
        let second = place(&mut g, &WORLD, B);
        let key = crate::loops::position(&g.state);
        let mark = g.state.mark();
        let plain = g.cards.id_of(&PLAIN);
        let world = g.cards.id_of(&WORLD);
        cantrip_api::Game::set_copy(&mut cx(&mut g), first, Copiable::of(plain));
        cantrip_api::Game::set_copy(&mut cx(&mut g), first, Copiable::of(world));
        assert_ne!(crate::loops::position(&g.state), key);
        g.state.rollback(mark);
        assert_eq!(crate::loops::position(&g.state), key);
        for id in [first, second] {
            g.state.update_obj(id, |o| {
                o.timestamp += 1000;
                o.world_since += 1000;
            });
        }
        assert_eq!(
            crate::loops::position(&g.state),
            key,
            "absolute clock magnitudes do not distinguish identical world order"
        );
    }
    /// What the layers derived while a question changes the position for the moment and reverts it (`ops::as_cast`) is set
    /// aside with the position (`GameState::set_derived_aside`): a World that enters once the state's count of changes is
    /// back where it was in the question still gets its interval - a "no World could be here" found in the question, for
    /// that count, is not kept.
    #[test]
    fn a_world_entering_after_a_set_aside_question_gets_its_interval() {
        let mut g = game();
        let aside = g.state.set_derived_aside();
        let mark = g.state.mark();
        place(&mut g, &PLAIN, A);
        g.state.revert(mark);
        g.state.restore_derived(aside);
        let world = place(&mut g, &WORLD, A);
        assert_ne!(g.state.obj(world).world_since, 0, "the World entered with an interval");
    }
    /// A World that stops being one ends its interval though nothing else on the battlefield is or could make a World: the
    /// interval it has is itself a reason to look (`StaticSources::worlds`), so becoming a World again later starts anew.
    #[test]
    fn a_lone_world_that_stops_being_one_ends_its_interval() {
        let mut g = game();
        let world = place(&mut g, &WORLD, A);
        assert_ne!(g.state.obj(world).world_since, 0);
        let plain = g.cards.id_of(&PLAIN);
        cantrip_api::Game::set_copy(&mut cx(&mut g), world, Copiable::of(plain));
        sync(&mut cx(&mut g));
        assert_eq!(g.state.obj(world).world_since, 0, "no longer a World");
    }
    /// The World intervals are kept only where something can make a World, so a static ability that does is marked: one
    /// that is not is refused as the layers apply it, in debug builds, rather than leaving a World with no interval.
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "makes_world")]
    fn a_static_ability_that_makes_a_world_unmarked_is_refused() {
        let mut g = game();
        let land = place(&mut g, &LAND, A);
        place(&mut g, &UNMARKED, A);
        cantrip_api::Query::chars(&g, land);
    }
}
