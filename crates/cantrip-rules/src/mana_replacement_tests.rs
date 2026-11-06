//! Replacement effects on the mana a permanent tapped for mana produces (CR 106.12b, 614.1a): the production is one event,
//! proposed as the mana ability resolves and before its tap is recorded, that each effect has one opportunity at (CR 614.5)
//! in the order the player who tapped it chooses (CR 616.1). A triggered mana ability's "additional" mana is no part of it
//! (CR 605.1b) and triggers on the mana as the effects left it (CR 106.12a). What the player has to pay with and what a
//! mana ability's row adds are judged through the same effects. Over test cards: Damping Sphere's, Mana Reflection's and
//! Contamination's effects, and Mana Flare's trigger.

use crate::actions::{can_afford, priority_options, replaced_mana_yields};
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

/// "If a land is tapped for two or more mana, it produces {C} instead of any other type and amount."
fn two_or_more_from_a_land(q: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::ManaProduction(m) if m.total() >= 2 && q.chars(m.permanent).is(CardType::Land))
}

fn one_colorless_instead(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::ManaProduction(m) = p {
        m.mana = ManaPool::default();
        m.mana.0[Mana::C as usize] = 1;
    }
}

/// "If you tap a permanent for mana, it produces twice as much of that mana instead."
fn you_tap_for_mana(_: &dyn Query, asker: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::ManaProduction(m) if m.player == asker.controller)
}

fn twice_as_much(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::ManaProduction(m) = p {
        m.mana = ManaPool(m.mana.0.map(|n| n * 2));
    }
}

/// "If a land is tapped for mana, it produces {B} instead of any other type and amount."
fn a_land_tapped(q: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::ManaProduction(m) if q.chars(m.permanent).is(CardType::Land))
}

fn one_black_instead(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::ManaProduction(m) = p {
        m.mana = ManaPool::default();
        m.mana.0[Mana::B as usize] = 1;
    }
}

/// "Whenever a player taps a land for mana, that player adds one mana of any type that land produced": the player and the
/// types produced in the data, the first of them added.
fn a_land_tapped_for_mana(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match *event {
        Event::TappedForMana { permanent, player, produced } if q.chars(permanent).is(CardType::Land) && !produced.is_empty() => {
            let types = (0..6).filter(|&m| produced.0[m] > 0).fold(0u32, |t, m| t | 1 << m);
            Some(Fired { subject: Target::Object(permanent), data: [player.0 as u32, types] })
        }
        _ => None,
    }
}

fn one_of_the_types_produced(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let mana = Mana::ALL.into_iter().find(|&m| item.data[1] & 1 << m as u32 != 0).expect("a type it produced");
    g.add_mana(PlayerId(item.data[0] as u8), mana, 1);
    Ok(())
}

static FILLER: CardDef = CardDef { name: "Mana Replacement Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static SPHERE: CardDef = CardDef {
    name: "Mana Replacement Sphere",
    types: TypeSet::of(&[CardType::Artifact]),
    replacements: &[replacement(ReplacementKind::ManaProduction, two_or_more_from_a_land, one_colorless_instead, "produces {C} instead")],
    ..CardDef::BLANK
};
static REFLECTION: CardDef = CardDef {
    name: "Mana Replacement Reflection",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ManaProduction, you_tap_for_mana, twice_as_much, "produces twice as much instead")],
    ..CardDef::BLANK
};
static CONTAMINATION: CardDef = CardDef {
    name: "Mana Replacement Contamination",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ManaProduction, a_land_tapped, one_black_instead, "produces {B} instead")],
    ..CardDef::BLANK
};
static FLARE: CardDef = CardDef {
    name: "Mana Replacement Flare",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(
        when(EventMask::TAPPED, a_land_tapped_for_mana),
        one_of_the_types_produced,
        "Whenever a player taps a land for mana, that player adds one mana of any type that land produced.",
    )
    .mana_ability(TriggeredMana::OneOfProduced)],
    ..CardDef::BLANK
};
/// "{T}: Add {C}{C}."
static TOMB: CardDef = CardDef {
    name: "Mana Replacement Tomb",
    types: TypeSet::of(&[CardType::Land]),
    abilities: &[mana_ability(TAP, add_n(Mana::C, 2), "{T}: Add {C}{C}.")],
    ..CardDef::BLANK
};
/// "{T}: Add {G}."
static GROVE: CardDef = CardDef {
    name: "Mana Replacement Grove",
    types: TypeSet::of(&[CardType::Land]),
    abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")],
    ..CardDef::BLANK
};
/// "{T}: Add one mana of any color."
static TOWER: CardDef = CardDef {
    name: "Mana Replacement Tower",
    types: TypeSet::of(&[CardType::Land]),
    abilities: &[mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color.")],
    ..CardDef::BLANK
};
/// "{T}: Add {C}{C}", an artifact: no land.
static ROCK: CardDef = CardDef {
    name: "Mana Replacement Rock",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add_n(Mana::C, 2), "{T}: Add {C}{C}.")],
    ..CardDef::BLANK
};

/// "{T}: Choose a player. That player adds {G}": the opponent, here.
fn the_opponent_adds_g(g: &mut dyn cantrip_api::Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(PlayerId(1 - asker.controller.0), Mana::G, 1);
    Ok(())
}
static SEARCHLIGHT: CardDef = CardDef {
    name: "Mana Replacement Searchlight",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add_by(the_opponent_adds_g), "{T}: Choose a player. That player adds {G}.")],
    ..CardDef::BLANK
};
/// "{T}, Sacrifice this artifact: Add three mana of any one color."
static LOTUS: CardDef = CardDef {
    name: "Mana Replacement Lotus",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP.and(SACRIFICE_THIS), add_any_color(3), "{T}, Sacrifice this artifact: Add three mana of any one color.")],
    ..CardDef::BLANK
};
/// "{T}: Add {U}{U}, {U}{B}, or {B}{B}": the two colors chosen as the mana is added, a question the tap asks as it resolves.
fn two_of_blue_and_black(g: &mut dyn cantrip_api::Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    let choices = ColorSet(ColorSet::of(Color::Blue).0 | ColorSet::of(Color::Black).0);
    let Answer::Colors(colors) =
        g.ask(asker.controller, RequestKind::ChooseColors { source: asker.source, choices, count: 2, purpose: ColorUse::AddMana })?
    else {
        return Ok(());
    };
    for c in colors {
        g.add_mana(asker.controller, c.mana(), 1);
    }
    Ok(())
}
static RUINS: CardDef = CardDef {
    name: "Mana Replacement Ruins",
    types: TypeSet::of(&[CardType::Land]),
    abilities: &[mana_ability(TAP, add_by(two_of_blue_and_black), "{T}: Add {U}{U}, {U}{B}, or {B}{B}.")],
    ..CardDef::BLANK
};

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([
        &FILLER, &SPHERE, &REFLECTION, &CONTAMINATION, &FLARE, &TOMB, &GROVE, &TOWER, &ROCK, &SEARCHLIGHT, &LOTUS, &RUINS,
    ])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.state.update_player(YOU, |p| p.turn_began = 1);
    g
}

fn put(g: &mut Game, def: &'static CardDef) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, Zone::BATTLEFIELD));
    let timestamp = g.state.timestamp();
    g.state.update_obj(id, |o| o.timestamp = timestamp);
    g.state.zone_push(Zone::BATTLEFIELD, id);
    g.state.see_statics();
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// Taps `source` for its mana from priority, naming `color` where it names one, and answers each order of replacement
/// effects asked with the first of `first` among the effects offered: the pool afterwards, and how many orders were asked.
fn tap(g: &mut Game, source: ObjectId, color: Option<Color>, first: &[ObjectId]) -> (ManaPool, usize) {
    g.run();
    let r = g.pending().cloned().expect("a question");
    let option = match &r.kind {
        RequestKind::Priority { options } => options.iter().find(|o| o.source == Some(source)).cloned(),
        _ => None,
    }
    .expect("its mana ability is offered");
    let mut action = Action::of(&option);
    action.color = color;
    g.answer(r.id, r.player, Answer::Act(action)).expect("a legal answer");
    let mut asked = 0;
    loop {
        g.run();
        let Some(r) = g.pending().cloned() else { break };
        if let RequestKind::ChooseCostPayment { choices, .. } = &r.kind {
            let chosen = choices.iter().position(|c| c.kind == CostPaymentKind::Tap).unwrap_or(0);
            g.answer(r.id, r.player, Answer::CostPayment(chosen)).expect("a legal cost order");
            continue;
        }
        let RequestKind::ChooseReplacement { sources, .. } = &r.kind else { break };
        assert_eq!(r.player, YOU, "the player who tapped it orders the effects (CR 616.1)");
        let pick = sources.iter().position(|s| first.contains(s)).expect("an effect to apply first");
        asked += 1;
        g.answer(r.id, r.player, Answer::Index(pick as u8)).expect("a legal answer");
    }
    (g.state.player(YOU).pool, asked)
}

fn pool(of: &[(Mana, u32)]) -> ManaPool {
    let mut p = ManaPool::default();
    for &(m, n) in of {
        p.0[m as usize] += n;
    }
    p
}

/// Damping Sphere: a land tapped for two mana produces {C}; one tapped for one, and a permanent that is no land, are not
/// changed.
#[test]
fn a_land_tapped_for_two_or_more_mana_produces_one_colorless_instead() {
    let mut g = game();
    put(&mut g, &SPHERE);
    let tomb = put(&mut g, &TOMB);
    assert_eq!(tap(&mut g, tomb, None, &[]), (pool(&[(Mana::C, 1)]), 0));
    let mut g = game();
    put(&mut g, &SPHERE);
    let grove = put(&mut g, &GROVE);
    let rock = put(&mut g, &ROCK);
    assert_eq!(tap(&mut g, grove, None, &[]).0, pool(&[(Mana::G, 1)]));
    assert_eq!(tap(&mut g, rock, None, &[]).0, pool(&[(Mana::G, 1), (Mana::C, 2)]));
}

/// Mana Reflection: twice as much of that mana, of the type named for one of any color.
#[test]
fn a_doubler_makes_twice_as_much_of_the_mana_produced() {
    let mut g = game();
    put(&mut g, &REFLECTION);
    let grove = put(&mut g, &GROVE);
    let tower = put(&mut g, &TOWER);
    assert_eq!(tap(&mut g, grove, None, &[]).0, pool(&[(Mana::G, 2)]));
    assert_eq!(tap(&mut g, tower, Some(Color::Red), &[]).0, pool(&[(Mana::G, 2), (Mana::R, 2)]));
}

/// CR 616.1, 616.1f, 614.5: the player orders the effects that apply, each judged again once one has applied. A land that taps
/// for {C}{C}: the Sphere first makes {C}, which the Reflection doubles; the Reflection first makes four, which the Sphere
/// turns to one. A land that taps for {G}: only the Reflection applies at first, and its {G}{G} then meets the Sphere -
/// nothing is asked.
#[test]
fn several_effects_on_one_production_apply_in_the_order_its_player_chooses() {
    let both = |first: fn(ObjectId, ObjectId) -> ObjectId| {
        let mut g = game();
        let sphere = put(&mut g, &SPHERE);
        let reflection = put(&mut g, &REFLECTION);
        let tomb = put(&mut g, &TOMB);
        tap(&mut g, tomb, None, &[first(sphere, reflection)])
    };
    assert_eq!(both(|sphere, _| sphere), (pool(&[(Mana::C, 2)]), 1));
    assert_eq!(both(|_, reflection| reflection), (pool(&[(Mana::C, 1)]), 1));
    let mut g = game();
    put(&mut g, &SPHERE);
    put(&mut g, &REFLECTION);
    let grove = put(&mut g, &GROVE);
    assert_eq!(tap(&mut g, grove, None, &[]), (pool(&[(Mana::C, 1)]), 0));
}

/// CR 605.1b, 106.12a: "adds one mana of any type that land produced" is a triggered mana ability, no replacement: it
/// triggers on the mana the effects left, and its own mana is not doubled (Mana Reflection's ruling). Contamination's {B}
/// for a Tower named white is what it sees, and so adds {B} (Contamination's ruling: affected in color, not in amount).
#[test]
fn a_triggered_additional_mana_sees_the_replaced_mana_and_is_not_replaced() {
    let mut g = game();
    put(&mut g, &REFLECTION);
    put(&mut g, &FLARE);
    let grove = put(&mut g, &GROVE);
    assert_eq!(tap(&mut g, grove, None, &[]).0, pool(&[(Mana::G, 3)]));
    let mut g = game();
    put(&mut g, &SPHERE);
    put(&mut g, &FLARE);
    let tomb = put(&mut g, &TOMB);
    assert_eq!(tap(&mut g, tomb, None, &[]).0, pool(&[(Mana::C, 2)]));
    let mut g = game();
    put(&mut g, &CONTAMINATION);
    put(&mut g, &FLARE);
    let tower = put(&mut g, &TOWER);
    assert_eq!(tap(&mut g, tower, Some(Color::White), &[]).0, pool(&[(Mana::B, 2)]));
    let history = g.log.iter().filter_map(|r| match r.event {
        Event::TappedForMana { permanent, produced, .. } if permanent == tower => Some(produced),
        _ => None,
    });
    assert_eq!(history.collect::<Vec<_>>(), vec![ManaTally([0, 0, 1, 0, 0, 0])], "the tap is recorded with the mana it produced");
}

/// What the player has to pay with (`available`) is judged through the same effects: two Tombs under the Sphere pay {2} and
/// not {3}; one Grove under the Reflection pays {G}{G}; under Contamination a Tower pays {B} and no {W}.
#[test]
fn payment_planning_counts_the_replaced_mana() {
    let mut g = game();
    let spell = put(&mut g, &FILLER);
    put(&mut g, &TOMB);
    put(&mut g, &TOMB);
    assert!(can_afford(&cx(&mut g), YOU, spell, &ManaCost { generic: 4, ..ManaCost::ZERO }));
    put(&mut g, &SPHERE);
    assert!(can_afford(&cx(&mut g), YOU, spell, &ManaCost { generic: 2, ..ManaCost::ZERO }));
    assert!(!can_afford(&cx(&mut g), YOU, spell, &ManaCost { generic: 3, ..ManaCost::ZERO }));

    let green = |n: u32| {
        let mut symbols = [0; 6];
        symbols[Mana::G as usize] = n;
        ManaCost { symbols, ..ManaCost::ZERO }
    };
    let mut g = game();
    let spell = put(&mut g, &FILLER);
    put(&mut g, &GROVE);
    assert!(!can_afford(&cx(&mut g), YOU, spell, &green(2)));
    put(&mut g, &REFLECTION);
    assert!(can_afford(&cx(&mut g), YOU, spell, &green(2)));

    let one = |m: Mana| {
        let mut symbols = [0; 6];
        symbols[m as usize] = 1;
        ManaCost { symbols, ..ManaCost::ZERO }
    };
    let mut g = game();
    let spell = put(&mut g, &FILLER);
    put(&mut g, &TOWER);
    assert!(can_afford(&cx(&mut g), YOU, spell, &one(Mana::W)));
    put(&mut g, &CONTAMINATION);
    assert!(can_afford(&cx(&mut g), YOU, spell, &one(Mana::B)));
    assert!(!can_afford(&cx(&mut g), YOU, spell, &one(Mana::W)));
}

/// What a mana ability's row adds (`replaced_mana_yields`): every outcome, for each type named, in each order; nothing where no
/// effect would change it.
#[test]
fn a_mana_abilitys_row_is_told_the_replaced_mana() {
    let mut g = game();
    let tomb = put(&mut g, &TOMB);
    let tower = put(&mut g, &TOWER);
    assert_eq!(replaced_mana_yields(&g, YOU, tomb, 0), None);
    put(&mut g, &SPHERE);
    put(&mut g, &REFLECTION);
    let mut outcomes = replaced_mana_yields(&g, YOU, tomb, 0).expect("changed").remove(0).1;
    outcomes.sort_by_key(|p| p.total());
    assert_eq!(outcomes, vec![pool(&[(Mana::C, 1)]), pool(&[(Mana::C, 2)])]);
    let yields = replaced_mana_yields(&g, YOU, tower, 0).expect("changed");
    assert_eq!(yields.len(), 5, "one for each color it may name");
    assert!(yields.iter().all(|(_, o)| o == &vec![pool(&[(Mana::C, 1)])]), "{yields:?}");
    // Offered still, and named a color as before.
    assert!(priority_options(&mut cx(&mut g), YOU).iter().any(|o| o.source == Some(tower) && !o.color_choices.is_empty()));
}

/// CR 106.12b: the whole production is the event, whoever's pool it goes to. Tapped by you for an opponent's {G}, "if you tap a
/// permanent for mana" doubles it, into the opponent's pool; yours gets none.
#[test]
fn mana_an_ability_adds_to_another_players_pool_is_replaced_too() {
    let mut g = game();
    put(&mut g, &REFLECTION);
    let searchlight = put(&mut g, &SEARCHLIGHT);
    assert_eq!(tap(&mut g, searchlight, None, &[]).0, ManaPool::default());
    assert_eq!(g.state.player(PlayerId(1)).pool, pool(&[(Mana::G, 2)]));
    assert_eq!(produced_by(&g, searchlight), vec![ManaTally([0, 0, 0, 0, 2, 0])], "the tap produced the mana it sent");
}

/// CR 106.12a: what a tap produced is all the mana its ability added, to any pool, with no replacement effect about too:
/// the opponent's {G} is what the Searchlight was tapped for.
#[test]
fn a_tap_produces_the_mana_it_adds_to_another_players_pool() {
    let mut g = game();
    let searchlight = put(&mut g, &SEARCHLIGHT);
    assert_eq!(tap(&mut g, searchlight, None, &[]).0, ManaPool::default());
    assert_eq!(g.state.player(PlayerId(1)).pool, pool(&[(Mana::G, 1)]));
    assert_eq!(produced_by(&g, searchlight), vec![ManaTally([0, 0, 0, 0, 1, 0])]);
}

/// What each tap of `permanent` produced, as the history records it (`Event::TappedForMana`).
fn produced_by(g: &Game, permanent: ObjectId) -> Vec<ManaTally> {
    g.log
        .iter()
        .filter_map(|r| match r.event {
            Event::TappedForMana { permanent: p, produced, .. } if p == permanent => Some(produced),
            _ => None,
        })
        .collect()
}

/// A cost that sacrifices the source with its {T} ("{T}, Sacrifice this artifact"): it is still tapped for mana (CR 106.12),
/// so its three red are doubled, read as it last existed (CR 608.2h).
#[test]
fn a_source_sacrificed_to_its_own_mana_ability_is_still_tapped_for_mana() {
    let mut g = game();
    put(&mut g, &REFLECTION);
    let lotus = put(&mut g, &LOTUS);
    assert_eq!(tap(&mut g, lotus, Some(Color::Red), &[]).0, pool(&[(Mana::R, 6)]));
    assert!(!g.state.battlefield().contains(&lotus), "it was sacrificed");
}

/// A filter land's colors are chosen as the mana is added, a question asked in the middle of the production: the activation is
/// run again with the answer, and the {U}{B} chosen is doubled whole.
#[test]
fn a_question_asked_in_the_middle_of_a_production_is_replayed_and_the_whole_production_replaced() {
    let mut g = game();
    put(&mut g, &REFLECTION);
    let ruins = put(&mut g, &RUINS);
    g.run();
    let r = g.pending().cloned().expect("a question");
    let RequestKind::Priority { options } = &r.kind else { panic!("priority") };
    let option = options.iter().find(|o| o.source == Some(ruins)).cloned().expect("its mana ability is offered");
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    g.run();
    let r = g.pending().cloned().expect("the colors are asked");
    assert!(matches!(r.kind, RequestKind::ChooseColors { count: 2, .. }), "{:?}", r.kind);
    assert_eq!(g.state.player(YOU).pool, ManaPool::default(), "nothing is added before the colors are chosen");
    g.answer(r.id, r.player, Answer::Colors(vec![Color::Blue, Color::Black])).expect("a legal answer");
    g.run();
    assert_eq!(g.state.player(YOU).pool, pool(&[(Mana::U, 2), (Mana::B, 2)]));
}

/// CR 106.7: the types a permanent could produce count a mana ability whose mana is its own effect's, every answer to its
/// question tried, and the replacement effects in any order: a filter land could produce {U} or {B}; under Contamination only
/// {B}; under the Sphere, whose two mana become {C}, only {C}.
#[test]
fn the_types_a_permanent_could_produce_count_its_effects_mana_through_the_replacements() {
    use crate::actions::producible_types;
    let blue_black = ColorSet(ColorSet::of(Color::Blue).0 | ColorSet::of(Color::Black).0);
    let mut g = game();
    let ruins = put(&mut g, &RUINS);
    assert_eq!(producible_types(&cx(&mut g), ruins), (blue_black, false));
    let contamination = put(&mut g, &CONTAMINATION);
    assert_eq!(producible_types(&cx(&mut g), ruins), (ColorSet::of(Color::Black), false));
    g.state.zone_remove(Zone::BATTLEFIELD, contamination);
    put(&mut g, &SPHERE);
    assert_eq!(producible_types(&cx(&mut g), ruins), (ColorSet::NONE, true));
}
