//! An emblem whose text names the object that made it (`Kept::Maker`, `Query::made_by`; CR 114.2, 114.4, 400.7, 607.2a, 614.12),
//! over test cards shaped as Tibalt, Cosmic Impostor is: "As this enters, you get an emblem with 'You may play cards exiled
//! with [this]'" and "Exile the top card of each player's library". No card in content has one yet, so no scenario can hold
//! this: Tibalt's own will.
//!
//! The emblem names the permanent as the new object it arrived as, under the player it entered under, for an entry from
//! anywhere, a token copy's and a back face's; each maker's exiles are its own emblem's; a maker that has left still has its
//! emblem and its cards, and one that returns is a new object with its own (the card's ruling); nothing is made of an entry
//! sent elsewhere or a maker that would have lost its abilities (CR 614.12); and all of it goes through the game-log tree
//! exactly (every answer here is also round-tripped by `Game::verify`).

use crate::actions::priority_options;
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static FILLER: CardDef = CardDef { name: "Emblem Maker Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// An instant for {0} with this effect.
const fn instant(name: &'static str, effect: Effect) -> CardDef {
    CardDef {
        name,
        cost: ManaCost::ZERO,
        has_cost: true,
        types: TypeSet::of(&[CardType::Instant]),
        spell: Some(SpellDef { targets: &[], effect }),
        ..CardDef::BLANK
    }
}

static SPELL: CardDef = instant("Emblem Maker Test Spell", nothing);

/// "You may play cards exiled with [what made this emblem]."
fn exiled_with_its_maker(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> bool {
    player == asker.controller && q.made_by(asker.source).is_some_and(|by| q.exiled_with(by).contains(&card))
}

static EMBLEM: CardDef = CardDef {
    name: "Emblem Maker Test Emblem",
    has_cost: false,
    exile_casts: &[ExileCast {
        text: "You may play cards exiled with its maker.",
        from: ZoneKind::Exile,
        allows: exiled_with_its_maker,
        additional: Cost::NONE,
        any_type: false,
        any_color: true,
        lands: true,
        life_for_mana_value: false,
        instead: None,
    }],
    ..CardDef::BLANK
};

/// "As this enters, you get an emblem."
fn emblem_as_it_enters(g: &mut dyn cantrip_api::Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.emblem = Some(g.card_id(&EMBLEM));
    Ok(())
}

/// "Exile the top card of each player's library."
fn exile_each_top(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let tops: Vec<ObjectId> = g.state().seats().filter_map(|p| g.state().zone(Zone::library(p)).first().copied()).collect();
    let by = g.linked(item);
    g.exile_with(&tops, by, false);
    Ok(())
}

const EXILE_EACH_TOP: u8 = 0;
const SACRIFICE: u8 = 1;

static MAKER: CardDef = CardDef {
    name: "Emblem Maker Test Maker",
    cost: ManaCost::ZERO,
    has_cost: true,
    types: TypeSet::of(&[CardType::Artifact]),
    enters: Some(emblem_as_it_enters),
    abilities: &[
        ability(cost("{0}"), exile_each_top, "{0}: Exile the top card of each player's library."),
        ability(SACRIFICE_THIS, nothing, "Sacrifice this artifact: Nothing."),
    ],
    ..CardDef::BLANK
};

/// A modal double-faced card's back face with a mana cost, a face of its own (CR 607.1b): Tibalt's shape.
static MODAL_BACK: CardDef = CardDef { name: "Emblem Maker Test Modal Back", ..MAKER };
static MODAL_FRONT: CardDef = CardDef {
    name: "Emblem Maker Test Modal Front",
    cost: ManaCost::ZERO,
    has_cost: true,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    back: Some(&MODAL_BACK),
    ..CardDef::BLANK
};

/// A transforming card's back face, printed on the whole card (CR 607.1b).
static WHOLE_BACK: CardDef = CardDef { name: "Emblem Maker Test Whole Back", has_cost: false, ..MAKER };
static WHOLE_FRONT: CardDef = CardDef {
    name: "Emblem Maker Test Whole Front",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    back: Some(&WHOLE_BACK),
    ..CardDef::BLANK
};

/// "As this enters, you get an emblem. If you do, put it into your graveyard instead of onto the battlefield."
fn emblem_then_elsewhere(g: &mut dyn cantrip_api::Game, entering: &mut Entering) -> Result<(), Suspend> {
    emblem_as_it_enters(g, entering)?;
    entering.instead = Some(Zone::graveyard(g.state().obj(entering.object).owner));
    Ok(())
}
static ELSEWHERE: CardDef = CardDef { name: "Emblem Maker Test Elsewhere", enters: Some(emblem_then_elsewhere), ..MAKER };

fn artifact(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, chars: &Chars) -> bool {
    chars.is(CardType::Artifact)
}
static HUMBLER: CardDef = CardDef {
    name: "Emblem Maker Test Humbler",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[loses_all_abilities(artifact, "Artifacts lose all abilities.")],
    ..CardDef::BLANK
};

/// "You get an emblem with 'You may play cards exiled with this artifact.'", as an ability resolves.
fn emblem_as_it_resolves(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let def = g.card_id(&EMBLEM);
    let by = g.linked(item);
    g.create_emblem_made_by(def, item.controller, by);
    Ok(())
}
static RESOLVER: CardDef = CardDef {
    name: "Emblem Maker Test Resolver",
    enters: None,
    abilities: &[
        ability(cost("{0}"), exile_each_top, "{0}: Exile the top card of each player's library."),
        ability(cost("{0}"), emblem_as_it_resolves, "{0}: You get an emblem."),
    ],
    ..MAKER
};

fn first_maker(g: &(impl Query + ?Sized)) -> Option<ObjectId> {
    g.state().battlefield().iter().copied().find(|&id| g.chars(id).is(CardType::Artifact))
}

/// "Create a token that's a copy of an artifact you control."
fn copy_a_maker(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(maker) = first_maker(g) {
        g.create_token_copies(maker, item.controller, 1)?;
    }
    Ok(())
}
static COPIER: CardDef = instant("Emblem Maker Test Copier", copy_a_maker);

/// "Put an artifact card from your graveyard onto the battlefield."
fn reanimate(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let card = g.state().zone(Zone::graveyard(item.controller)).iter().copied().find(|&id| g.chars(id).is(CardType::Artifact));
    if let Some(card) = card {
        g.put_onto_battlefield(card, item.controller, false)?;
    }
    Ok(())
}
static REANIMATOR: CardDef = instant("Emblem Maker Test Reanimator", reanimate);

/// "Put an artifact card from your graveyard onto the battlefield transformed."
fn reanimate_transformed(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let card = g.state().zone(Zone::graveyard(item.controller)).iter().copied().find(|&id| g.chars(id).is(CardType::Artifact));
    if let Some(card) = card {
        g.put_onto_battlefield_transformed(card, item.controller, false)?;
    }
    Ok(())
}
static TRANSFORMER: CardDef = instant("Emblem Maker Test Transformer", reanimate_transformed);

/// "Exile the top card of your library", linked to nothing.
fn exile_your_top(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(&top) = g.state().zone(Zone::library(item.controller)).first() {
        g.exile(top);
    }
    Ok(())
}
static PLAIN_EXILE: CardDef = instant("Emblem Maker Test Plain Exile", exile_your_top);

/// Your precombat main phase of turn 3 with priority and an empty stack, every answer round-tripped through the game-log
/// tree (`Game::verify`). Whatever a test puts in place, it puts there before the first answer, so the game log holds the
/// whole of what happens after.
fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([
        &FILLER,
        &SPELL,
        &EMBLEM,
        &MAKER,
        &MODAL_FRONT,
        &MODAL_BACK,
        &WHOLE_FRONT,
        &WHOLE_BACK,
        &ELSEWHERE,
        &HUMBLER,
        &RESOLVER,
        &COPIER,
        &REANIMATOR,
        &TRANSFORMER,
        &PLAIN_EXILE,
    ])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.number = 3;
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.verify = true;
    // The top of your library, what your makers exile of yours.
    for _ in 0..6 {
        let id = g.state.push_obj(Obj::new(g.cards.id_of(&SPELL), YOU, Zone::library(YOU)));
        g.state.zone_insert(Zone::library(YOU), 0, id);
    }
    g
}

fn put(g: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let mut obj = Obj::new(g.cards.id_of(def), YOU, zone);
    obj.timestamp = g.state.timestamp();
    let id = g.state.push_obj(obj);
    g.state.zone_push(zone, id);
    if !def.statics.is_empty() {
        g.state.see_statics();
    }
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

fn options(g: &mut Game) -> Vec<ActionOption> {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    options.clone()
}

fn pay(g: &mut Game) {
    loop {
        let r = next(g);
        let RequestKind::PayCosts { options, .. } = &r.kind else { break };
        let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("it costs nothing").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
    }
}

fn resolve(g: &mut Game) {
    loop {
        let r = next(g);
        match &r.kind {
            RequestKind::Priority { .. } if g.state.stack.is_empty() && g.state.triggers.is_empty() && r.player == YOU => break,
            RequestKind::Priority { .. } => g.answer(r.id, r.player, Answer::Pass).expect("a legal answer"),
            other => panic!("unexpected {other:?}"),
        }
    }
}

/// Takes the option, pays nothing for it, and lets what it put on the stack resolve.
fn take(g: &mut Game, option: &ActionOption) {
    let r = next(g);
    g.answer(r.id, r.player, Answer::Act(Action::of(option))).expect("a legal answer");
    pay(g);
    resolve(g);
}

/// Casts `card` from your hand as `way`, and lets it resolve: what it became.
fn cast_as(g: &mut Game, card: ObjectId, way: u8) -> ObjectId {
    let all = options(g);
    let option = all.iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(card) && o.index == way).expect("it may be cast").clone();
    take(g, &option);
    let mut new = card;
    while let Some(next) = g.state.obj(new).became {
        new = next;
    }
    new
}

fn cast(g: &mut Game, card: ObjectId) -> ObjectId {
    cast_as(g, card, 0)
}

fn activate(g: &mut Game, source: ObjectId, index: u8) {
    let all = options(g);
    let option = all.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(source) && o.index == index).expect("offered").clone();
    take(g, &option);
}

/// The ability of `object`'s printed `def`, as linked abilities know it.
fn own(g: &Game, object: ObjectId, def: &'static CardDef) -> Linked {
    Linked { object, text: g.cards.linked_card(g.cards.id_of(def)), acquired: Acquired::OWN }
}

fn emblems(g: &Game) -> Vec<ObjectId> {
    g.state.zone(Zone::COMMAND).to_vec()
}

/// The emblems that let you play `card` as you have priority: the permission of each option to play it.
fn played_by(g: &mut Game, card: ObjectId) -> Vec<ObjectId> {
    let all = priority_options(&mut cx(g), YOU);
    let mut by: Vec<ObjectId> =
        all.iter().filter(|o| o.source == Some(card) && matches!(o.kind, ActionKind::PlayLand | ActionKind::CastSpell)).filter_map(|o| o.granted_by.map(|g| g.object)).collect();
    by.sort();
    by
}

/// What `maker`, of the text `def`, exiles of each player's library top: your spell, then their land.
fn exile_tops(g: &mut Game, maker: ObjectId, def: &'static CardDef) -> (ObjectId, ObjectId) {
    let (mine, theirs) = (g.state.zone(Zone::library(YOU))[0], g.state.zone(Zone::library(THEM))[0]);
    activate(g, maker, EXILE_EACH_TOP);
    let (mine, theirs) = (g.state.obj(mine).became.expect("exiled"), g.state.obj(theirs).became.expect("exiled"));
    let with = Query::exiled_with(g, own(g, maker, def));
    assert!(with.contains(&mine) && with.contains(&theirs));
    (mine, theirs)
}

#[test]
fn as_it_enters_the_emblem_names_the_permanent_it_arrived_as() {
    let mut g = game();
    let card = put(&mut g, &MAKER, Zone::hand(YOU));
    let all = options(&mut g);
    let option = all.iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(card)).expect("it may be cast").clone();
    let r = next(&mut g);
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    pay(&mut g);
    assert!(emblems(&g).is_empty(), "not while it is a spell");
    resolve(&mut g);
    let spell = g.state.obj(card).became.expect("cast");
    let permanent = g.state.obj(spell).became.expect("resolved");
    let &[emblem] = emblems(&g).as_slice() else { panic!("one emblem") };
    // The object it arrived as, no object it was before (CR 400.7).
    assert_eq!(Query::made_by(&g, emblem), Some(own(&g, permanent, &MAKER)));
    assert!(g.state.obj(permanent).zone.is(ZoneKind::Battlefield));
    assert_eq!((g.state.obj(emblem).owner, g.state.obj(emblem).controller), (YOU, YOU));
    assert!(g.state.obj(emblem).zone.is(ZoneKind::Command));
}

/// "You": the player it entered under, from anywhere (CR 114.2, 614.12).
#[test]
fn the_emblem_goes_to_the_player_it_enters_under() {
    let mut g = game();
    let card = put(&mut g, &MAKER, Zone::graveyard(YOU));
    let permanent = cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), card, THEM, false).expect("no question");
    let &[emblem] = emblems(&g).as_slice() else { panic!("one emblem") };
    assert_eq!(Query::made_by(&g, emblem), Some(own(&g, permanent, &MAKER)));
    assert_eq!((g.state.obj(emblem).owner, g.state.obj(emblem).controller), (THEM, THEM));
}

/// A token copy has the "as this enters" of what it copies (CR 707.9a), and is an object of its own: its emblem names it, with
/// the copied text its abilities link by, and its exiles are its own (the card's ruling).
#[test]
fn a_token_copy_of_a_maker_makes_an_emblem_of_its_own() {
    let mut g = game();
    let (maker, copier) = (put(&mut g, &MAKER, Zone::hand(YOU)), put(&mut g, &COPIER, Zone::hand(YOU)));
    let original = cast(&mut g, maker);
    cast(&mut g, copier);
    let token = *g.state.battlefield().iter().find(|&&id| g.state.obj(id).token).expect("the token");
    let all = emblems(&g);
    assert_eq!(all.len(), 2);
    assert_eq!(Query::made_by(&g, all[0]), Some(own(&g, original, &MAKER)));
    assert_eq!(Query::made_by(&g, all[1]), Some(own(&g, token, &MAKER)));
    let (mine, theirs) = exile_tops(&mut g, token, &MAKER);
    assert_eq!(played_by(&mut g, mine), vec![all[1]], "only the token's own emblem");
    assert_eq!(played_by(&mut g, theirs), vec![all[1]]);
}

/// The text a back face's abilities link by, which is what its emblem must hold to find what they exile (CR 607.1b): a modal
/// face's own, cast as that face - Tibalt's.
#[test]
fn a_modal_back_faces_emblem_finds_what_its_abilities_exile() {
    let mut g = game();
    let card = put(&mut g, &MODAL_FRONT, Zone::hand(YOU));
    let walker = cast_as(&mut g, card, WAY_BACK_FACE);
    let &[emblem] = emblems(&g).as_slice() else { panic!("one emblem") };
    assert_eq!(g.cards.linked_card(g.cards.id_of(&MODAL_BACK)), g.cards.id_of(&MODAL_BACK), "a face of its own");
    assert_eq!(Query::made_by(&g, emblem), Some(own(&g, walker, &MODAL_BACK)));
    let (mine, _) = exile_tops(&mut g, walker, &MODAL_BACK);
    assert_eq!(played_by(&mut g, mine), vec![emblem]);
}

/// And a transforming card's whole, either face being that card.
#[test]
fn a_transforming_back_faces_emblem_finds_what_its_abilities_exile() {
    let mut g = game();
    put(&mut g, &WHOLE_FRONT, Zone::graveyard(YOU));
    let transformer = put(&mut g, &TRANSFORMER, Zone::hand(YOU));
    cast(&mut g, transformer);
    let permanent = first_maker(&g).expect("it entered");
    let &[emblem] = emblems(&g).as_slice() else { panic!("one emblem") };
    assert_eq!(g.cards.linked_card(g.cards.id_of(&WHOLE_BACK)), g.cards.id_of(&WHOLE_FRONT), "printed on the whole card");
    assert_eq!(Query::made_by(&g, emblem), Some(own(&g, permanent, &WHOLE_FRONT)));
    let (mine, _) = exile_tops(&mut g, permanent, &WHOLE_FRONT);
    assert_eq!(played_by(&mut g, mine), vec![emblem]);
}

/// Tibalt's shape: two makers of one name, each with its emblem. What each exiled, from either library, is played only by the
/// permission of its own (CR 607.2a; the card's ruling), and nothing else in exile is covered.
#[test]
fn two_makers_exiles_stay_apart() {
    let mut g = game();
    let (a, b, plain) = (put(&mut g, &MAKER, Zone::hand(YOU)), put(&mut g, &MAKER, Zone::hand(YOU)), put(&mut g, &PLAIN_EXILE, Zone::hand(YOU)));
    let (one, other) = (cast(&mut g, a), cast(&mut g, b));
    let all = emblems(&g);
    assert_eq!(all.len(), 2);
    let (first, second) = (all[0], all[1]);
    assert_eq!(Query::made_by(&g, first), Some(own(&g, one, &MAKER)));
    assert_eq!(Query::made_by(&g, second), Some(own(&g, other, &MAKER)));
    let (one_spell, one_land) = exile_tops(&mut g, one, &MAKER);
    let (other_spell, other_land) = exile_tops(&mut g, other, &MAKER);
    assert_eq!(played_by(&mut g, one_spell), vec![first]);
    assert_eq!(played_by(&mut g, one_land), vec![first], "a card of another's library, a land");
    assert_eq!(played_by(&mut g, other_spell), vec![second]);
    assert_eq!(played_by(&mut g, other_land), vec![second]);
    let top = g.state.zone(Zone::library(YOU))[0];
    cast(&mut g, plain);
    let exiled = g.state.obj(top).became.expect("exiled");
    assert_eq!(played_by(&mut g, exiled), Vec::<ObjectId>::new(), "exiled by nothing");
}

/// The ruling: "allows you to play cards exiled with that specific Tibalt ... even after that Tibalt leaves the battlefield",
/// and "a different Tibalt ... it's a new object (even if it's represented by the same card)".
#[test]
fn a_maker_that_left_still_has_its_emblem_and_a_returned_one_is_another_object() {
    let mut g = game();
    let (card, reanimator) = (put(&mut g, &MAKER, Zone::hand(YOU)), put(&mut g, &REANIMATOR, Zone::hand(YOU)));
    let maker = cast(&mut g, card);
    let first = emblems(&g)[0];
    let (spell, land) = exile_tops(&mut g, maker, &MAKER);
    activate(&mut g, maker, SACRIFICE);
    assert!(g.state.obj(g.state.obj(maker).became.expect("sacrificed")).zone.is(ZoneKind::Graveyard));
    // The record stands, the links stand.
    assert_eq!(Query::made_by(&g, first), Some(own(&g, maker, &MAKER)));
    assert_eq!(g.state.obj(spell).exiled.map(|l| l.by), Some(maker));
    assert_eq!(played_by(&mut g, spell), vec![first]);
    assert_eq!(played_by(&mut g, land), vec![first]);

    // The same card returned: a new object, a new emblem, cards of its own that the first never reaches.
    cast(&mut g, reanimator);
    let back = first_maker(&g).expect("it returned");
    assert_ne!(back, maker);
    let all = emblems(&g);
    assert_eq!(all.len(), 2);
    let second = all[1];
    assert_eq!(Query::made_by(&g, second), Some(own(&g, back, &MAKER)));
    let (newer, _) = exile_tops(&mut g, back, &MAKER);
    assert_eq!(played_by(&mut g, newer), vec![second]);
    assert_eq!(played_by(&mut g, spell), vec![first], "the old maker's stay the old emblem's");

    // Played by it: gone from exile, a new object neither reaches (CR 400.7).
    let option = options(&mut g).into_iter().find(|o| o.source == Some(spell) && o.kind == ActionKind::CastSpell).expect("castable");
    take(&mut g, &option);
    assert_eq!(Query::exiled_with(&g, own(&g, maker, &MAKER)), vec![land]);
}

/// {U}{C}, to pay for from exile.
static COSTLY: CardDef = CardDef {
    name: "Emblem Maker Test Costly",
    types: TypeSet::of(&[CardType::Sorcery]),
    cost: ManaCost { symbols: [0, 1, 0, 0, 0, 1], ..ManaCost::ZERO },
    has_cost: true,
    spell: Some(SpellDef { targets: &[], effect: nothing }),
    ..CardDef::BLANK
};

/// Whether the card exiled with an emblem's maker may be cast with this pool: "you may spend mana as though it were mana of any
/// color to cast those spells" (CR 609.4b, 105.2a), from the command zone (CR 114.4). Red pays {U} and not {C}.
fn costly_castable(pool: [u32; 6], exiled_with_the_maker: bool) -> bool {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&COSTLY, &EMBLEM, &MAKER])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let maker = game.state.push_obj(Obj::new(table.id_of(&MAKER), YOU, Zone::GONE));
    let card = game.state.push_obj(Obj::new(table.id_of(&COSTLY), YOU, Zone::exile(YOU)));
    game.state.zone_push(Zone::exile(YOU), card);
    let by = Linked { object: maker, text: table.id_of(&MAKER), acquired: Acquired::OWN };
    if exiled_with_the_maker {
        game.state.update_obj(card, |o| o.exiled = Some(ExileLink::new(by, Zone::library(YOU), false)));
    }
    cantrip_api::Game::create_emblem_made_by(&mut cx(&mut game), table.id_of(&EMBLEM), YOU, by);
    game.state.update_player(YOU, |p| p.pool = ManaPool(pool));
    game.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    priority_options(&mut cx(&mut game), YOU).iter().any(|o| o.kind == ActionKind::CastSpell && o.source == Some(card))
}

#[test]
fn an_emblems_permission_spends_mana_as_any_color_and_no_more() {
    const RED_RED: [u32; 6] = [0, 0, 0, 2, 0, 0];
    const RED_COLORLESS: [u32; 6] = [0, 0, 0, 1, 0, 1];
    assert!(costly_castable(RED_COLORLESS, true), "red pays {{U}} beside colorless for {{C}}");
    assert!(!costly_castable(RED_RED, true), "no color stands for {{C}}");
    // Colorless mana, too, is spent as though it were any color (Mycosynth Lattice's ruling): it pays {U}.
    assert!(costly_castable([0, 0, 0, 0, 0, 2], true), "colorless pays {{U}} as though it were blue");
    assert!(!costly_castable(RED_COLORLESS, false), "the permission is for the maker's cards alone");
}

/// Replacement effects are of the permanent it would be (CR 614.12): one that would have lost its abilities makes none, and
/// an entry sent elsewhere arrives nowhere, so it names nothing.
#[test]
fn no_emblem_comes_of_an_entry_that_is_not_one_or_of_abilities_lost() {
    let mut g = game();
    let card = put(&mut g, &ELSEWHERE, Zone::graveyard(YOU));
    let _ = cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), card, YOU, false);
    assert!(emblems(&g).is_empty(), "it went elsewhere");
    assert!(g.state.battlefield().is_empty());

    let mut g = game();
    put(&mut g, &HUMBLER, Zone::BATTLEFIELD);
    let card = put(&mut g, &MAKER, Zone::graveyard(YOU));
    cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), card, YOU, false).expect("no question");
    assert!(emblems(&g).is_empty(), "it would have had no such ability");

    // A face-down permanent has no abilities (CR 708.2a, 708.3): manifested, it gets no emblem.
    let mut g = game();
    let card = put(&mut g, &MAKER, Zone::graveyard(YOU));
    let manifested = cantrip_api::Game::put_onto_battlefield_face_down(&mut cx(&mut g), card, YOU, FaceDown::Manifested, false).expect("no question");
    assert!(manifested.is_some_and(|id| g.state.obj(id).face_down), "it entered face down");
    assert!(emblems(&g).is_empty(), "a face-down permanent has no \"as this enters\"");
}

/// As an ability resolves, the emblem takes that ability's link, covering what was exiled before it was made.
#[test]
fn an_emblem_made_as_an_ability_resolves_names_that_ability() {
    let mut g = game();
    let card = put(&mut g, &RESOLVER, Zone::hand(YOU));
    let maker = cast(&mut g, card);
    assert!(emblems(&g).is_empty());
    let (spell, _) = exile_tops(&mut g, maker, &RESOLVER);
    activate(&mut g, maker, 1);
    let &[emblem] = emblems(&g).as_slice() else { panic!("one emblem") };
    assert_eq!(Query::made_by(&g, emblem), Some(own(&g, maker, &RESOLVER)));
    assert_eq!(played_by(&mut g, spell), vec![emblem]);

    // An ability its object acquired otherwise is linked otherwise (CR 607.5), and the emblem says so.
    let acquired = Linked { acquired: Acquired { by: AcquiredBy::Copy, stamp: 7, part: 0 }, ..own(&g, maker, &RESOLVER) };
    let def = g.cards.id_of(&EMBLEM);
    let other = cantrip_api::Game::create_emblem_made_by(&mut cx(&mut g), def, YOU, acquired);
    assert_eq!(Query::made_by(&g, other), Some(acquired));
    assert_eq!(played_by(&mut g, spell), vec![emblem]);
}

/// The game-log tree: going back above the entry takes the emblem away, above the exile takes the link away, and down again
/// restores all of it exactly, with the maker gone as it was (`Game::goto`, `tree.rs`).
#[test]
fn the_emblem_and_its_links_go_through_the_game_tree() {
    let mut g = game();
    let card = put(&mut g, &MAKER, Zone::hand(YOU));
    g.run();
    let before_entry = (g.at, g.state.clone());
    let maker = cast(&mut g, card);
    let emblem = emblems(&g)[0];
    let entered = (g.at, g.state.clone());
    let (spell, land) = exile_tops(&mut g, maker, &MAKER);
    let exiled = (g.at, g.state.clone());
    activate(&mut g, maker, SACRIFICE);
    let left = (g.at, g.state.clone());

    g.goto(entered.0);
    assert!(g.state == entered.1, "back above the exile");
    assert_eq!(Query::exiled_with(&g, own(&g, maker, &MAKER)), Vec::<ObjectId>::new());
    assert_eq!(Query::made_by(&g, emblem), Some(own(&g, maker, &MAKER)));

    g.goto(before_entry.0);
    assert!(g.state == before_entry.1, "back above the entry");
    assert!(emblems(&g).is_empty());

    g.goto(left.0);
    assert!(g.state == left.1, "down to the maker gone");
    assert_eq!(emblems(&g), vec![emblem]);
    assert_eq!(Query::made_by(&g, emblem), Some(own(&g, maker, &MAKER)));
    assert_eq!(Query::exiled_with(&g, own(&g, maker, &MAKER)), vec![spell, land]);
    assert_eq!(played_by(&mut g, spell), vec![emblem]);
    assert_eq!(played_by(&mut g, land), vec![emblem]);

    g.goto(exiled.0);
    assert!(g.state == exiled.1, "up to the maker still there");
    assert!(g.state.obj(maker).zone.is(ZoneKind::Battlefield));
    assert_eq!(Query::exiled_with(&g, own(&g, maker, &MAKER)), vec![spell, land]);
}
