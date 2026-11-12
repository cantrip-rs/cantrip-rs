//! A cast taken back (CR 733.1) with the mana abilities activated while paying for it, over test cards: each is reversed
//! but one that may not be - one that moved a card out of a library or a hand, or made any other hidden card known (the
//! owner's ruling) - and one whose mana paid for that one (CR 733.1). What stays keeps its costs paid and its mana in the
//! pool; the spell goes back where it was.

use crate::{Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

static FILLER: CardDef = CardDef { name: "Takeback Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "{T}: Add {R}."
static MOUNTAIN: CardDef = CardDef {
    name: "Takeback Test Mountain",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    supertypes: Supertypes { basic: true, legendary: false, snow: false, world: false },
    subtypes: SubtypeSet::of(&[Subtype::Mountain]),
    basic_land_types: &[Mana::R],
    ..CardDef::BLANK
};

/// "{T}: Add {G}."
static FOREST: CardDef = CardDef {
    name: "Takeback Test Forest",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    supertypes: Supertypes { basic: true, legendary: false, snow: false, world: false },
    subtypes: SubtypeSet::of(&[Subtype::Forest]),
    basic_land_types: &[Mana::G],
    ..CardDef::BLANK
};

/// A 2/2 for {R}{R}.
static OGRE: CardDef = CardDef {
    name: "Takeback Test Ogre",
    cost: mana("{R}{R}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

static COSTED_OGRE: CardDef = CardDef {
    name: "Takeback Test Costed Ogre", cost: mana("{R}{R}"), types: TypeSet::of(&[CardType::Creature]),
    power: Some(2), toughness: Some(2),
    additional_cost: Some(Cost { life: 3, exile_from_graveyard: 1, ..Cost::NONE }), ..CardDef::BLANK
};

static CONVOKE_OGRE: CardDef = CardDef {
    name: "Takeback Test Convoke Ogre", cost: mana("{1}{R}"), types: TypeSet::of(&[CardType::Creature]),
    power: Some(2), toughness: Some(2), pay_with: PayWith { convoke: true, ..PayWith::NONE },
    additional_cost: Some(Cost { life: 3, exile_from_graveyard: 1, ..Cost::NONE }), ..CardDef::BLANK
};

fn red_and_draw(g: &mut dyn cantrip_api::Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::R, 1);
    g.draw(asker.controller, 1);
    Ok(())
}

/// "{T}, Sacrifice this artifact: Add {R}. Draw a card." (Chromatic Sphere's shape.)
static SPHERE: CardDef = CardDef {
    name: "Takeback Test Sphere",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP.and(SACRIFICE_THIS), add(Mana::R).by(red_and_draw), "{T}, Sacrifice this artifact: Add {R}. Draw a card.")],
    ..CardDef::BLANK
};

static LIFE_SPHERE: CardDef = CardDef {
    name: "Takeback Test Life Sphere", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP.and(SACRIFICE_THIS).and(life(3)), add(Mana::R), "{T}, sacrifice this, pay 3 life: Add {R}.")],
    ..CardDef::BLANK
};

/// "{1}, {T}, Sacrifice this artifact: Add {R}. Draw a card." (Chromatic Sphere, red only.)
static PAID_SPHERE: CardDef = CardDef {
    name: "Takeback Test Paid Sphere",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(cost("{1}, {T}").and(SACRIFICE_THIS), add(Mana::R).by(red_and_draw), "{1}, {T}, Sacrifice this artifact: Add {R}. Draw a card.")],
    ..CardDef::BLANK
};

/// "{2}, {T}, Sacrifice this artifact: Add {R}. Draw a card."
static DEAR_SPHERE: CardDef = CardDef {
    name: "Takeback Test Dear Sphere",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(cost("{2}, {T}").and(SACRIFICE_THIS), add(Mana::R).by(red_and_draw), "{2}, {T}, Sacrifice this artifact: Add {R}. Draw a card.")],
    ..CardDef::BLANK
};

/// "{1}, {T}: Add {R}{R}."
static FILTER: CardDef = CardDef {
    name: "Takeback Test Filter",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(cost("{1}, {T}"), add_n(Mana::R, 2), "{1}, {T}: Add {R}{R}.")],
    ..CardDef::BLANK
};

fn red_and_draw_or_mill(g: &mut dyn cantrip_api::Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::R, 1);
    if g.state().stack.is_empty() {
        g.mill(asker.controller, 1);
    } else {
        g.draw(asker.controller, 1);
    }
    Ok(())
}

/// "{T}, Sacrifice this artifact: Add {R}. If a spell is on the stack, draw a card; otherwise mill a card." No printed mana
/// ability reads the stack: this one is made to, so that its replay, with the spell taken back, does something else.
static FICKLE_SPHERE: CardDef = CardDef {
    name: "Takeback Test Fickle Sphere",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(
        TAP.and(SACRIFICE_THIS),
        add(Mana::R).by(red_and_draw_or_mill),
        "{T}, Sacrifice this artifact: Add {R}. If a spell is on the stack, draw a card; otherwise mill a card.",
    )],
    ..CardDef::BLANK
};

/// A 5/5 for {R}{R}{R}{R}{R}.
static TITAN: CardDef = CardDef {
    name: "Takeback Test Titan",
    cost: mana("{R}{R}{R}{R}{R}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(5),
    toughness: Some(5),
    ..CardDef::BLANK
};

fn red_and_return(g: &mut dyn cantrip_api::Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::R, 1);
    g.return_to_hand(&[asker.source]);
    Ok(())
}

/// "{T}: Add {R}. Return this artifact to its owner's hand.": a card moved into a hidden zone, which may be undone.
static BOUNCER: CardDef = CardDef {
    name: "Takeback Test Bouncer",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::R).by(red_and_return), "{T}: Add {R}. Return this artifact to its owner's hand.")],
    ..CardDef::BLANK
};

/// "Discard your hand, Sacrifice this artifact: Add {R}{R}{R}." (Lion's Eye Diamond's shape.)
static DIAMOND: CardDef = CardDef {
    name: "Takeback Test Diamond",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(DISCARD_HAND.and(SACRIFICE_THIS), add_n(Mana::R, 3), "Discard your hand, Sacrifice this artifact: Add {R}{R}{R}.")],
    ..CardDef::BLANK
};

/// "Exile this card from your hand: Add {R}." (Simian Spirit Guide's shape.)
static GUIDE: CardDef = CardDef {
    name: "Takeback Test Guide",
    types: TypeSet::of(&[CardType::Creature]),
    cost: mana("{2}{R}"),
    power: Some(2),
    toughness: Some(2),
    abilities: &[mana_ability(EXILE_THIS, add(Mana::R), "Exile this card from your hand: Add {R}.").from(ZoneKind::Hand)],
    ..CardDef::BLANK
};

fn any_spell(_: &dyn Query, _: &Asker, _: ObjectId) -> bool {
    true
}

/// "You may cast spells from the top of your library." (Future Sight's, for its spells.)
static SIGHT: CardDef = CardDef {
    name: "Takeback Test Sight",
    types: TypeSet::of(&[CardType::Enchantment]),
    play_rules: &[PlayRule::LibraryTopSpells(any_spell, None)],
    ..CardDef::BLANK
};

/// A 4/4 for {R}{R}{R}{R}.
static GIANT: CardDef = CardDef {
    name: "Takeback Test Giant",
    cost: mana("{R}{R}{R}{R}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(4),
    ..CardDef::BLANK
};

fn any_creature(q: &dyn Query, _: &Asker, id: ObjectId) -> bool { q.chars(id).is(CardType::Creature) }
fn giant_creature(q: &dyn Query, _: &Asker, id: ObjectId) -> bool { q.chars(id).name == "Takeback Test Giant" }
static OVERLAP_OGRE: CardDef = CardDef {
    name: "Takeback Test Overlap Ogre", cost: mana("{R}"), types: TypeSet::of(&[CardType::Creature]),
    power: Some(2), toughness: Some(2), additional_cost: Some(Cost::sacrifice(1, any_creature)),
    optional_cost: Some(OptionalCost::new(Cost::sacrifice(1, giant_creature))),
    labels: &["Without optional cost", "Sacrifice a giant"], ..CardDef::BLANK
};

fn game() -> Game {
    let table: &'static CardTable =
        Box::leak(Box::new(CardTable::new([&FILLER, &MOUNTAIN, &FOREST, &OGRE, &COSTED_OGRE, &CONVOKE_OGRE, &OVERLAP_OGRE, &SPHERE, &LIFE_SPHERE, &PAID_SPHERE, &BOUNCER, &DIAMOND, &GUIDE, &SIGHT, &GIANT, &DEAR_SPHERE, &FILTER, &TITAN, &FICKLE_SPHERE])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    g.verify = true;
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    // The opening hands go back to the libraries' bottoms: each test deals its own.
    for p in [YOU, PlayerId(1)] {
        for id in g.state.zone(Zone::hand(p)).to_vec() {
            g.state.zone_remove(Zone::hand(p), id);
            g.state.update_obj(id, |o| o.zone = Zone::library(p));
            g.state.zone_push(Zone::library(p), id);
        }
    }
    g.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g
}

fn put(g: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, zone));
    g.state.zone_push(zone, id);
    id
}

/// A card put on top of a library.
fn put_on_top(g: &mut Game, def: &'static CardDef) -> ObjectId {
    let zone = Zone::library(YOU);
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, zone));
    g.state.zone_insert(zone, 0, id);
    id
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

fn options(r: &Request) -> Vec<ActionOption> {
    match &r.kind {
        RequestKind::Priority { options } | RequestKind::PayCosts { options, .. } => options.clone(),
        other => panic!("expected priority or a payment, got {other:?}"),
    }
}

fn choose(g: &mut Game, pick: impl Fn(&ActionOption) -> bool, what: &str) {
    loop {
        let r = next(g);
        if let RequestKind::ChooseCostPayment { choices, .. } = &r.kind {
            let chosen = choices.iter().position(|c| c.kind == CostPaymentKind::Tap)
                .or_else(|| choices.iter().position(|c| c.kind == CostPaymentKind::Mana))
                .or_else(|| choices.iter().position(|c| c.kind != CostPaymentKind::Sacrifice)).unwrap_or(0);
            g.answer(r.id, r.player, Answer::CostPayment(chosen)).expect("a legal cost order");
            continue;
        }
        let offered = options(&r);
        if let Some(o) = offered.iter().find(|o| pick(o)) {
            g.answer(r.id, r.player, Answer::Act(Action::of(o))).expect("a legal answer");
            return;
        }
        if matches!(r.kind, RequestKind::PayCosts { mana_abilities: true, .. })
            && let Some(confirm) = offered.iter().find(|o| o.kind == ActionKind::ConfirmCasting)
        {
            g.answer(r.id, r.player, Answer::Act(Action::of(confirm))).expect("close the mana window");
            continue;
        }
        panic!("{what} offered in {:?}", r.kind);
    }
}

fn cast(g: &mut Game, card: ObjectId) {
    choose(g, |o| o.kind == ActionKind::CastSpell && o.source == Some(card), "the cast");
}

/// Activates `source`'s mana ability while paying.
fn tap(g: &mut Game, source: ObjectId) {
    choose(g, |o| o.kind == ActionKind::ActivateAbility && o.source == Some(source), "the mana ability");
    loop {
        let r = next(g);
        let RequestKind::ChooseCostPayment { source: paying, choices } = &r.kind else { break };
        if *paying != source { break; }
        let chosen = choices.iter().position(|c| c.kind == CostPaymentKind::Tap)
            .or_else(|| choices.iter().position(|c| c.kind == CostPaymentKind::Mana))
            .or_else(|| choices.iter().position(|c| c.kind != CostPaymentKind::Sacrifice))
            .unwrap_or(0);
        g.answer(r.id, r.player, Answer::CostPayment(chosen)).unwrap();
    }
}

/// Closes the mana ability window where it can be closed with mana still owed (some other way to pay the rest); without
/// one, it stays open until the mana covers the cost or the cast is taken back.
fn close_mana_window(g: &mut Game) {
    let r = next(g);
    if options(&r).iter().any(|o| o.kind == ActionKind::ConfirmCasting) {
        choose(g, |o| o.kind == ActionKind::ConfirmCasting, "closing the mana ability window");
    }
}

fn cancel(g: &mut Game) {
    choose(g, |o| o.kind == ActionKind::CancelCasting, "taking the cast back");
    // Priority again: the cast is gone.
    assert!(matches!(next(g).kind, RequestKind::Priority { .. }), "priority again once the cast is taken back");
}

fn pool(g: &Game, mana: Mana) -> u32 {
    g.state.player(YOU).pool.get(mana)
}

fn in_zone(g: &Game, card: &'static CardDef, zone: Zone) -> usize {
    g.state.zone(zone).iter().filter(|&&id| g.cards.get(g.state.obj(id).card).name == card.name).count()
}




#[test]
fn a_mana_ability_that_drew_is_not_taken_back_with_the_cast() {
    let mut g = game();
    let ogre = put(&mut g, &OGRE, Zone::hand(YOU));
    let sphere = put(&mut g, &SPHERE, Zone::BATTLEFIELD);
    let mountain = put(&mut g, &MOUNTAIN, Zone::BATTLEFIELD);
    let top = g.state.zone(Zone::library(YOU))[0];
    let library = g.state.zone(Zone::library(YOU)).len();
    cast(&mut g, ogre);
    tap(&mut g, sphere);
    cancel(&mut g);
    assert_eq!(g.state.zone(Zone::hand(YOU)).len(), 2, "the Ogre is back, and the card drawn stays");
    assert!(g.state.zone(Zone::hand(YOU)).contains(&ogre), "the Ogre goes back to the hand as the object it was");
    assert_eq!(g.state.zone(Zone::library(YOU)).len(), library - 1, "the draw is not undone");
    assert!(!g.state.zone(Zone::library(YOU)).contains(&top), "the card drawn is not put back on top");
    assert_eq!(in_zone(&g, &SPHERE, Zone::graveyard(YOU)), 1, "the Sphere stays sacrificed");
    assert_eq!(in_zone(&g, &SPHERE, Zone::BATTLEFIELD), 0);
    assert_eq!(pool(&g, Mana::R), 1, "its mana stays in the pool");
    assert!(!g.state.obj(mountain).tapped, "the Mountain was never tapped");
    assert!(g.log.iter().any(|r| matches!(r.event, Event::Drew { player: YOU, .. })), "the history keeps the draw");
    assert!(g.state.stack.is_empty(), "nothing is cast");
}

/// The control: a mana ability that only moved a card into a hidden zone (its own source, returned to the hand) is reversed
/// with the cast, as any other.
#[test]
fn a_mana_ability_that_only_returned_a_card_to_the_hand_is_taken_back() {
    let mut g = game();
    let ogre = put(&mut g, &OGRE, Zone::hand(YOU));
    let bouncer = put(&mut g, &BOUNCER, Zone::BATTLEFIELD);
    put(&mut g, &MOUNTAIN, Zone::BATTLEFIELD);
    cast(&mut g, ogre);
    tap(&mut g, bouncer);
    assert_eq!(in_zone(&g, &BOUNCER, Zone::hand(YOU)), 1, "returned as its ability resolved");
    cancel(&mut g);
    assert_eq!(g.state.zone(Zone::hand(YOU)), &[ogre], "only the Ogre is in the hand");
    assert!(g.state.zone(Zone::BATTLEFIELD).contains(&bouncer), "the Bouncer is back on the battlefield, the object it was");
    assert!(!g.state.obj(bouncer).tapped, "and untapped");
    assert_eq!(pool(&g, Mana::R), 0, "its mana is gone with it");
}

/// CR 733.1: a mana ability whose mana paid for one that is not reversed is not reversed either; one whose mana only
/// waited in the pool is. The Mountain's {R} paid the Sphere's {1}: it stays tapped. The Forest's {G} paid nothing: it
/// is untapped and its mana gone. The Sphere's own {1} was asked for (two kinds in the pool) and is paid again without
/// asking.
#[test]
fn the_mana_ability_that_paid_for_a_kept_one_is_kept_and_the_rest_reversed() {
    let mut g = game();
    let ogre = put(&mut g, &OGRE, Zone::hand(YOU));
    let sphere = put(&mut g, &PAID_SPHERE, Zone::BATTLEFIELD);
    let mountain = put(&mut g, &MOUNTAIN, Zone::BATTLEFIELD);
    let forest = put(&mut g, &FOREST, Zone::BATTLEFIELD);
    let library = g.state.zone(Zone::library(YOU)).len();
    cast(&mut g, ogre);
    tap(&mut g, forest);
    tap(&mut g, mountain);
    tap(&mut g, sphere);
    // The Sphere's {1}: the Mountain's {R}, not the Forest's {G}.
    choose(&mut g, |o| o.kind == ActionKind::AssignMana && o.mana == Some(Mana::R), "the {R} for the Sphere's {1}");
    cancel(&mut g);
    assert_eq!(g.state.zone(Zone::library(YOU)).len(), library - 1, "the draw stays");
    assert_eq!(in_zone(&g, &PAID_SPHERE, Zone::graveyard(YOU)), 1, "the Sphere stays sacrificed");
    assert!(g.state.obj(mountain).tapped, "the Mountain's mana paid for the Sphere: it is not reversed");
    assert!(!g.state.obj(forest).tapped, "the Forest's mana paid for nothing kept: it is reversed");
    assert_eq!(pool(&g, Mana::G), 0, "the Forest's {{G}} is gone");
    assert_eq!(pool(&g, Mana::R), 1, "the Sphere's {{R}} is left, the Mountain's spent on the Sphere");
    assert!(g.state.zone(Zone::hand(YOU)).contains(&ogre));
}

/// A mana ability that discarded the hand stays (the cards left a hidden zone), and discarded the hand it discarded then:
/// not the spell, which was on the stack, and is back in the hand. The Diamond's {R}{R}{R} leaves the Giant one short.
#[test]
fn a_discarded_hand_stays_discarded_and_the_spell_goes_back_to_the_hand() {
    let mut g = game();
    let first = put(&mut g, &FILLER, Zone::hand(YOU));
    let giant = put(&mut g, &GIANT, Zone::hand(YOU));
    let last = put(&mut g, &FILLER, Zone::hand(YOU));
    let diamond = put(&mut g, &DIAMOND, Zone::BATTLEFIELD);
    put(&mut g, &MOUNTAIN, Zone::BATTLEFIELD);
    cast(&mut g, giant);
    tap(&mut g, diamond);
    cancel(&mut g);
    assert_eq!(g.state.zone(Zone::hand(YOU)), &[giant], "the Giant is back in the hand, the rest discarded");
    assert!(!g.state.zone(Zone::graveyard(YOU)).contains(&first) && !g.state.zone(Zone::graveyard(YOU)).contains(&last), "they are new objects there");
    assert_eq!(in_zone(&g, &FILLER, Zone::graveyard(YOU)), 2, "both cards discarded stay discarded");
    assert_eq!(in_zone(&g, &DIAMOND, Zone::graveyard(YOU)), 1, "the Diamond stays sacrificed");
    assert_eq!(pool(&g, Mana::R), 3, "its {{R}}{{R}}{{R}} stays in the pool");
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::Discarded { player: YOU, .. })).count(), 2);
}

/// A card exiled from the hand for its mana stays exiled: it left a hidden zone.
#[test]
fn a_card_exiled_from_the_hand_for_mana_stays_exiled() {
    let mut g = game();
    let ogre = put(&mut g, &OGRE, Zone::hand(YOU));
    let guide = put(&mut g, &GUIDE, Zone::hand(YOU));
    put(&mut g, &MOUNTAIN, Zone::BATTLEFIELD);
    put(&mut g, &MOUNTAIN, Zone::BATTLEFIELD);
    cast(&mut g, ogre);
    tap(&mut g, guide);
    cancel(&mut g);
    assert_eq!(g.state.zone(Zone::hand(YOU)), &[ogre]);
    assert_eq!(in_zone(&g, &GUIDE, Zone::exile(YOU)), 1, "the Guide stays exiled");
    assert_eq!(pool(&g, Mana::R), 1, "its {{R}} stays in the pool");
}

/// A spell cast from the top of a library goes back there, and the card the mana ability drew from under it stays drawn:
/// the draw is replayed with the spell set aside, so it draws the card it drew.
#[test]
fn a_spell_cast_from_the_library_goes_back_on_top_of_it_and_the_card_drawn_from_under_it_stays_drawn() {
    let mut g = game();
    put(&mut g, &SIGHT, Zone::BATTLEFIELD);
    let sphere = put(&mut g, &SPHERE, Zone::BATTLEFIELD);
    put(&mut g, &MOUNTAIN, Zone::BATTLEFIELD);
    let under = g.state.zone(Zone::library(YOU))[0];
    let ogre = put_on_top(&mut g, &OGRE);
    let library = g.state.zone(Zone::library(YOU)).len();
    cast(&mut g, ogre);
    tap(&mut g, sphere);
    cancel(&mut g);
    assert_eq!(g.state.zone(Zone::library(YOU))[0], ogre, "the Ogre is back on top");
    assert_eq!(g.state.zone(Zone::library(YOU)).len(), library - 1, "one card fewer: the one drawn");
    assert!(!g.state.zone(Zone::library(YOU)).contains(&under), "the card under the Ogre is the one drawn");
    let hand = g.state.zone(Zone::hand(YOU));
    assert_eq!(hand.len(), 1);
    assert_eq!(g.cards.get(g.state.obj(hand[0]).card).name, FILLER.name, "the card drawn is in the hand, not the Ogre");
    assert_eq!(pool(&g, Mana::R), 1);
}

/// The mana ability activated to pay for one that stays is part of it: the Mountain tapped while the Sphere's {1} was paid
/// stays tapped, its mana spent on the Sphere.
#[test]
fn a_mana_ability_activated_within_a_kept_one_is_kept_with_it() {
    let mut g = game();
    let ogre = put(&mut g, &OGRE, Zone::hand(YOU));
    let sphere = put(&mut g, &PAID_SPHERE, Zone::BATTLEFIELD);
    let mountain = put(&mut g, &MOUNTAIN, Zone::BATTLEFIELD);
    let forest = put(&mut g, &FOREST, Zone::BATTLEFIELD);
    cast(&mut g, ogre);
    tap(&mut g, sphere);
    // The Sphere's {1}, from the Mountain activated for it.
    tap(&mut g, mountain);
    close_mana_window(&mut g);
    cancel(&mut g);
    assert_eq!(in_zone(&g, &PAID_SPHERE, Zone::graveyard(YOU)), 1, "the Sphere stays sacrificed");
    assert!(g.state.obj(mountain).tapped, "the Mountain paid for it: it stays tapped");
    assert!(!g.state.obj(forest).tapped);
    assert_eq!(pool(&g, Mana::R), 1, "the Sphere's {{R}} is in the pool, the Mountain's spent");
    assert_eq!(g.state.zone(Zone::hand(YOU)).len(), 2, "the Ogre and the card drawn");
}

/// A mana ability given up part way is part of the action given up, and reversed; one that stays, activated to pay for it,
/// is activated again alone. The Guide's {R} went towards the Dear Sphere's {2}, then the cast was taken back from inside
/// that payment: the Sphere is back, untapped, nothing drawn; the Guide stays exiled, its {R} in the pool.
#[test]
fn a_kept_mana_ability_inside_one_given_up_stays_alone() {
    let mut g = game();
    let ogre = put(&mut g, &OGRE, Zone::hand(YOU));
    let guide = put(&mut g, &GUIDE, Zone::hand(YOU));
    let sphere = put(&mut g, &DEAR_SPHERE, Zone::BATTLEFIELD);
    put(&mut g, &MOUNTAIN, Zone::BATTLEFIELD);
    put(&mut g, &MOUNTAIN, Zone::BATTLEFIELD);
    let library = g.state.zone(Zone::library(YOU)).len();
    cast(&mut g, ogre);
    tap(&mut g, sphere);
    tap(&mut g, guide);
    cancel(&mut g);
    assert!(g.state.zone(Zone::BATTLEFIELD).contains(&sphere), "the Sphere's activation is reversed");
    assert!(!g.state.obj(sphere).tapped);
    assert_eq!(g.state.zone(Zone::library(YOU)).len(), library, "nothing drawn");
    assert_eq!(in_zone(&g, &GUIDE, Zone::exile(YOU)), 1, "the Guide stays exiled");
    assert_eq!(pool(&g, Mana::R), 1, "its {{R}} stays in the pool");
    assert_eq!(g.state.zone(Zone::hand(YOU)), &[ogre]);
}


/// A mana ability whose doing a kept one went on to undo is kept too: the Bouncer returned itself to the hand, and the
/// Diamond discarded that hand, the Bouncer with it. Both stay: the Bouncer card in the graveyard, its {R} and the Diamond's
/// {R}{R}{R} in the pool, the Titan back in the hand.
#[test]
fn a_mana_ability_whose_card_a_kept_one_moved_is_kept() {
    let mut g = game();
    let titan = put(&mut g, &TITAN, Zone::hand(YOU));
    let bouncer = put(&mut g, &BOUNCER, Zone::BATTLEFIELD);
    let diamond = put(&mut g, &DIAMOND, Zone::BATTLEFIELD);
    let mountain = put(&mut g, &MOUNTAIN, Zone::BATTLEFIELD);
    cast(&mut g, titan);
    tap(&mut g, bouncer);
    tap(&mut g, diamond);
    cancel(&mut g);
    assert_eq!(g.state.zone(Zone::hand(YOU)), &[titan]);
    assert_eq!(in_zone(&g, &BOUNCER, Zone::graveyard(YOU)), 1, "the Bouncer, returned then discarded, stays discarded");
    assert_eq!(in_zone(&g, &DIAMOND, Zone::graveyard(YOU)), 1);
    assert_eq!(pool(&g, Mana::R), 4, "the Bouncer's {{R}} and the Diamond's {{R}}{{R}}{{R}}");
    assert!(!g.state.obj(mountain).tapped);
}

/// An object a kept mana ability made is the object a later one names: the Sphere drew the Guide, and the Guide was exiled
/// from the hand for its {R}. Both stay, the Guide exiled, their {R}{R} in the pool.
#[test]
fn a_card_drawn_by_a_kept_mana_ability_then_exiled_for_mana_stays_exiled() {
    let mut g = game();
    let giant = put(&mut g, &GIANT, Zone::hand(YOU));
    let sphere = put(&mut g, &SPHERE, Zone::BATTLEFIELD);
    for _ in 0..3 {
        put(&mut g, &MOUNTAIN, Zone::BATTLEFIELD);
    }
    put_on_top(&mut g, &GUIDE);
    cast(&mut g, giant);
    tap(&mut g, sphere);
    let drawn = *g.state.zone(Zone::hand(YOU)).last().expect("the Guide, drawn");
    tap(&mut g, drawn);
    cancel(&mut g);
    assert_eq!(g.state.zone(Zone::hand(YOU)), &[giant]);
    assert_eq!(in_zone(&g, &GUIDE, Zone::exile(YOU)), 1, "the Guide, drawn and exiled, stays exiled");
    assert_eq!(in_zone(&g, &SPHERE, Zone::graveyard(YOU)), 1);
    assert_eq!(pool(&g, Mana::R), 2, "the Sphere's {{R}} and the Guide's");
    assert!(g.state.zone(Zone::BATTLEFIELD).iter().all(|&id| !g.state.obj(id).tapped), "no Mountain tapped");
}


/// A replay is held to what it replays: a mana ability that reads the stack draws while the spell is there, and would mill
/// once the spell is taken back. The replay is refused - which no printed card can bring about (`take_back`) - and the
/// checked build says so.
#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "did not activate again as it had")]
fn a_replay_that_does_something_else_is_refused() {
    let mut g = game();
    let ogre = put(&mut g, &OGRE, Zone::hand(YOU));
    let sphere = put(&mut g, &FICKLE_SPHERE, Zone::BATTLEFIELD);
    put(&mut g, &MOUNTAIN, Zone::BATTLEFIELD);
    cast(&mut g, ogre);
    tap(&mut g, sphere);
    choose(&mut g, |o| o.kind == ActionKind::CancelCasting, "taking the cast back");
}


#[test]
fn mandatory_source_tap_precedes_its_own_sacrifice_without_an_order_question() {
    let mut g = game();
    let source = put(&mut g, &SPHERE, Zone::BATTLEFIELD);
    let library = g.state.zone(Zone::library(YOU)).len();
    let r = next(&mut g);
    let ability = options(&r).into_iter().find(|o| o.source == Some(source) && o.kind == ActionKind::ActivateAbility).unwrap();
    g.answer(r.id, r.player, Answer::Act(Action::of(&ability))).unwrap();
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "only tap then sacrifice can finish: {:?}", r.kind);
    assert!(g.state.obj(source).tapped);
    assert!(g.state.obj(source).became.is_some_and(|id| g.state.obj(id).zone == Zone::graveyard(YOU)));
    assert_eq!(g.state.zone(Zone::library(YOU)).len(), library - 1);
    assert_eq!(pool(&g, Mana::R), 1);
}

#[test]
fn source_tap_dependency_keeps_independent_life_payment_available() {
    let mut g = game();
    let source = put(&mut g, &LIFE_SPHERE, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let ability = options(&r).into_iter().find(|o| o.source == Some(source) && o.kind == ActionKind::ActivateAbility).unwrap();
    g.answer(r.id, r.player, Answer::Act(Action::of(&ability))).unwrap();
    // The source is tapped first, then the life paid and the source sacrificed: no order of them could pay otherwise.
    assert!(matches!(next(&mut g).kind, RequestKind::Priority { .. }));
    assert_eq!(g.state.player(YOU).life, 17);
    assert_eq!(pool(&g, Mana::R), 1);
    assert!(g.state.obj(source).tapped);
    assert!(g.state.obj(source).became.is_some_and(|id| g.state.obj(id).zone == Zone::graveyard(YOU)));
}

/// Answers the question of which cost to pay next with the one `pick` finds, where the payment asks it (CR 601.2h); where
/// the order cannot matter it is not asked, and `r` is the next question already.
fn pay_first(g: &mut Game, r: Request, pick: impl Fn(&[CostPaymentChoice]) -> Option<usize>) -> Request {
    match &r.kind {
        RequestKind::ChooseCostPayment { choices, .. } => {
            let at = pick(choices).unwrap_or(0);
            g.answer(r.id, r.player, Answer::CostPayment(at)).unwrap();
            next(g)
        }
        _ => r,
    }
}


#[test]
fn committed_convoke_is_reversed_while_an_irreversible_mana_ability_remains() {
    let mut g = game();
    let spell = put(&mut g, &CONVOKE_OGRE, Zone::hand(YOU));
    let grave_card = put(&mut g, &FILLER, Zone::graveyard(YOU));
    let sphere = put(&mut g, &SPHERE, Zone::BATTLEFIELD);
    let giant = put(&mut g, &GIANT, Zone::BATTLEFIELD);
    let library = g.state.zone(Zone::library(YOU)).len();
    cast(&mut g, spell);
    tap(&mut g, sphere);
    let r = next(&mut g);
    let r = pay_first(&mut g, r, |choices| choices.iter().position(|c| c.kind == CostPaymentKind::Mana));
    let tap = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::TapToPay && o.source == Some(giant)).unwrap().clone(), other => panic!("{other:?}") };
    g.answer(r.id, r.player, Answer::Act(Action::of(&tap))).unwrap();
    let r = next(&mut g);
    let r = pay_first(&mut g, r, |choices| choices.iter().position(|c| c.kind == CostPaymentKind::CommitHelpers));
    let r = pay_first(&mut g, r, |choices| choices.iter().position(|c| c.kind == CostPaymentKind::Life));
    g.answer(r.id, r.player, Answer::Cancel).unwrap();
    assert!(matches!(next(&mut g).kind, RequestKind::Priority { .. }));
    assert_eq!(g.state.player(YOU).life, 20);
    assert!(!g.state.obj(giant).tapped);
    assert!(g.state.zone(Zone::hand(YOU)).contains(&spell));
    assert!(g.state.zone(Zone::graveyard(YOU)).contains(&grave_card));
    assert_eq!(g.state.zone(Zone::library(YOU)).len(), library - 1, "the draw remains");
    assert_eq!(in_zone(&g, &SPHERE, Zone::graveyard(YOU)), 1);
    assert_eq!(pool(&g, Mana::R), 1);
    assert!(!g.events().iter().any(|r| matches!(r.event, Event::CostObjectPaid { receipt, .. } if g.state.cost_receipts[receipt as usize].origin == Some(CostOrigin::ManaTotal))));
}

#[test]
fn irreversible_mana_is_kept_after_an_involuntary_failed_cast() {
    let mut g = game();
    let spell = put(&mut g, &OVERLAP_OGRE, Zone::hand(YOU));
    let giant = put(&mut g, &GIANT, Zone::BATTLEFIELD);
    let ogre = put(&mut g, &OGRE, Zone::BATTLEFIELD);
    let sphere = put(&mut g, &SPHERE, Zone::BATTLEFIELD);
    let library = g.state.zone(Zone::library(YOU)).len();
    cast(&mut g, spell);
    let r = next(&mut g);
    g.answer(r.id, r.player, Answer::Modes(vec![1])).unwrap();
    tap(&mut g, sphere);
    let r = next(&mut g);
    let r = pay_first(&mut g, r, |choices| choices.iter().position(|c| c.label.contains("additional cost of")));
    g.answer(r.id, r.player, Answer::Cards(vec![giant])).unwrap();
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "the missing optional sacrifice reverses the cast: {:?}", r.kind);
    assert!(g.state.zone(Zone::hand(YOU)).contains(&spell));
    assert!(g.state.zone(Zone::BATTLEFIELD).contains(&giant));
    assert!(g.state.zone(Zone::BATTLEFIELD).contains(&ogre));
    assert_eq!(g.state.zone(Zone::library(YOU)).len(), library - 1);
    assert_eq!(in_zone(&g, &SPHERE, Zone::graveyard(YOU)), 1);
    assert_eq!(pool(&g, Mana::R), 1);
    assert!(g.state.stack.is_empty());
    assert!(!g.events().iter().any(|r| matches!(r.event, Event::CostObjectPaid { .. })));
}



/// What paid for a mana ability activated within a kept one is kept too: the Forest's {G} paid the Filter's {1}, whose
/// {R}{R} paid the Dear Sphere's {2}. The Forest stays tapped, the Filter tapped, the Sphere sacrificed.
#[test]
fn what_paid_for_a_mana_ability_within_a_kept_one_is_kept() {
    let mut g = game();
    let ogre = put(&mut g, &OGRE, Zone::hand(YOU));
    let sphere = put(&mut g, &DEAR_SPHERE, Zone::BATTLEFIELD);
    let filter = put(&mut g, &FILTER, Zone::BATTLEFIELD);
    let forest = put(&mut g, &FOREST, Zone::BATTLEFIELD);
    put(&mut g, &MOUNTAIN, Zone::BATTLEFIELD);
    cast(&mut g, ogre);
    tap(&mut g, forest);
    tap(&mut g, sphere);
    tap(&mut g, filter);
    close_mana_window(&mut g);
    close_mana_window(&mut g);
    cancel(&mut g);
    assert_eq!(in_zone(&g, &DEAR_SPHERE, Zone::graveyard(YOU)), 1, "the Sphere stays sacrificed");
    assert!(g.state.obj(filter).tapped, "the Filter paid for it");
    assert!(g.state.obj(forest).tapped, "the Forest paid for the Filter");
    assert_eq!(pool(&g, Mana::R), 1, "the Sphere's {{R}}");
    assert_eq!(pool(&g, Mana::G), 0);
}
