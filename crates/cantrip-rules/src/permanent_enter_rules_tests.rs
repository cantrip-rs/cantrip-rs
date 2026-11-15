//! A static ability that keeps some permanents from entering the battlefield, judged as they would exist there
//! (`CardDef::permanent_enter_rules`, CR 101.2, 614.17), over test cards shaped like Worms of the Earth's "lands can't enter
//! the battlefield": a card is judged as what it would be there, not as the card it is where it is (CR 614.17d); a token
//! that would be a land is not created; an entry that a replacement effect makes into a land's can't happen once it does,
//! a permanent spell's then put into its owner's graveyard (CR 608.3e); and no other replacement effect replaces an entry
//! that can't happen (CR 614.17c). The scenarios show the rest with Worms of the Earth itself.

use crate::ops;
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

/// "Lands can't enter the battlefield."
fn lands_cant_enter(q: &dyn Query, _: &Asker, permanent: ObjectId) -> bool {
    !q.chars(permanent).is(CardType::Land)
}

static WORMS: CardDef = CardDef {
    name: "Entry Ban Test Worms",
    types: TypeSet::of(&[CardType::Enchantment]),
    permanent_enter_rules: &[lands_cant_enter],
    ..CardDef::BLANK
};

fn your_artifact(q: &dyn LayerQuery, source: ObjectId, id: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Artifact) && q.state().obj(id).controller == q.state().obj(source).controller
}

fn add_land(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.types = c.types.with(CardType::Land);
}

/// "Artifacts you control are lands."
static LANDER: CardDef = CardDef {
    name: "Entry Ban Test Lander",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::Type, your_artifact, add_land, "")],
    ..CardDef::BLANK
};

fn under_an_opponent(_: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.to.is(ZoneKind::Battlefield) && z.controller != asker.controller)
}

fn under_you(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.controller = P1;
    }
}

/// "Permanents your opponents would put onto the battlefield enter under your control instead."
static USURPER: CardDef = CardDef {
    name: "Entry Ban Test Usurper",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, under_an_opponent, under_you, "")],
    ..CardDef::BLANK
};

fn nontoken_creature_entering(q: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.to.is(ZoneKind::Battlefield) && !q.state().obj(z.object).token && q.chars(z.object).is(CardType::Creature))
}

fn exile_it(q: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.to = Zone::exile(q.state().obj(z.object).owner);
    }
}

/// "If a nontoken creature would enter the battlefield, exile it instead" (Containment Priest's, without "and it wasn't cast").
static PRIEST: CardDef = CardDef {
    name: "Entry Ban Test Priest",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, nontoken_creature_entering, exile_it, "")],
    ..CardDef::BLANK
};

static RELIC: CardDef = CardDef { name: "Entry Ban Test Relic", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };
static BEAR: CardDef = CardDef {
    name: "Entry Ban Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};
static ARBOR: CardDef = CardDef {
    name: "Entry Ban Test Arbor",
    types: TypeSet::of(&[CardType::Land, CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};
static WASTE: CardDef = CardDef { name: "Entry Ban Test Waste", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn as_a_bear(g: &mut dyn cantrip_api::Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.copy = Some(Copiable::of(g.card_id(&BEAR)));
    Ok(())
}

fn as_a_waste(g: &mut dyn cantrip_api::Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.copy = Some(Copiable::of(g.card_id(&WASTE)));
    Ok(())
}

/// A land that "enters as a copy of" the Bear.
static MIMIC_LAND: CardDef = CardDef { name: "Entry Ban Test Mimic Land", enters_as_copy: Some(as_a_bear), ..WASTE };
/// An artifact that "enters as a copy of" the Waste, as a Clone copying Dryad Arbor would.
static MIMIC_RELIC: CardDef = CardDef { name: "Entry Ban Test Mimic Relic", enters_as_copy: Some(as_a_waste), ..RELIC };

/// How many times the Shifting Land's copy replacement has been applied.
static SHIFTS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn shift_as_a_bear(g: &mut dyn cantrip_api::Game, entering: &mut Entering) -> Result<(), Suspend> {
    SHIFTS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    as_a_bear(g, entering)
}

fn as_the_shifting_land(g: &mut dyn cantrip_api::Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.copy = Some(Copiable::of(g.card_id(&SHIFTING_LAND)));
    Ok(())
}

/// A land with its own "enters as a copy" (a Vesuva), and an artifact that enters as a copy of it (a Clone copying an
/// animated Vesuva).
static SHIFTING_LAND: CardDef = CardDef { name: "Entry Ban Test Shifting Land", enters_as_copy: Some(shift_as_a_bear), ..WASTE };
static SHIFTING_MIMIC: CardDef = CardDef { name: "Entry Ban Test Shifting Mimic", enters_as_copy: Some(as_the_shifting_land), ..RELIC };

/// "It's a land."
static A_LAND: EntersAs = EntersAs { types: Some(TypeSet::of(&[CardType::Land])), ..EntersAs::NONE };
/// "It's an enchantment."
static AN_ENCHANTMENT: EntersAs = EntersAs { types: Some(TypeSet::of(&[CardType::Enchantment])), ..EntersAs::NONE };

/// P0's precombat main phase with the Worms on P0's battlefield.
fn game() -> (Game, &'static CardTable) {
    let table: &'static CardTable =
        Box::leak(Box::new(CardTable::new([&WORMS, &LANDER, &USURPER, &PRIEST, &RELIC, &BEAR, &ARBOR, &WASTE, &MIMIC_LAND, &MIMIC_RELIC, &SHIFTING_LAND, &SHIFTING_MIMIC])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    game.state.update_turn(|t| {
        t.active = P0;
        t.priority = P0;
        t.set_step(Step::PrecombatMain);
    });
    put(&mut game, table, &WORMS, Zone::BATTLEFIELD, P0);
    (game, table)
}

fn put(game: &mut Game, table: &CardTable, def: &'static CardDef, zone: Zone, controller: PlayerId) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table.id_of(def), controller, zone));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = timestamp);
    game.state.zone_push(zone, id);
    game.state.see_statics();
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None }
}

/// A token copy of a land would be a land on the battlefield, so it is not created (CR 614.17d, 111.7); a token copy of a
/// creature is.
#[test]
fn a_token_that_would_be_a_land_is_not_created() {
    let (mut game, table) = game();
    let mut cx = cx(&mut game);
    let lands = ops::create_token_copies(&mut cx, Copiable::of(table.id_of(&WASTE)), P0, 1).expect("nothing is asked");
    assert!(lands.is_empty());
    let bears = ops::create_token_copies(&mut cx, Copiable::of(table.id_of(&BEAR)), P0, 1).expect("nothing is asked");
    assert_eq!(bears.len(), 1);
    assert!(cx.s().battlefield().contains(&bears[0]));
}

/// Judged as it would exist on the battlefield, not as the card it is where it is (CR 614.17d), unlike an `EnterRule`: an
/// artifact card put onto the battlefield as a land can't enter and stays in the graveyard; a land card put there as an
/// enchantment (no land) can.
#[test]
fn a_card_is_judged_as_what_it_would_be_on_the_battlefield() {
    let (mut game, table) = game();
    let relic = put(&mut game, table, &RELIC, Zone::graveyard(P0), P0);
    let waste = put(&mut game, table, &WASTE, Zone::graveyard(P0), P0);
    let mut cx = cx(&mut game);
    let becomes = |what| Becomes { what, card: table.id_of(&WORMS), words: Words::NONE };
    assert_eq!(ops::put_onto_battlefield_becoming(&mut cx, relic, P0, becomes(&A_LAND)).expect("nothing is asked"), None);
    assert!(cx.s().zone(Zone::graveyard(P0)).contains(&relic));
    let entered = ops::put_onto_battlefield_becoming(&mut cx, waste, P0, becomes(&AN_ENCHANTMENT)).expect("nothing is asked");
    assert!(entered.is_some_and(|id| cx.s().battlefield().contains(&id)));
}

/// P1's Usurper has P0's Relic enter under P1's control, where P1's "artifacts you control are lands" would make it a land:
/// once that replacement has applied, its entry can't happen (CR 614.17c-d), and it stays in P0's hand. Without the Usurper
/// it enters under P0, no land.
#[test]
fn an_entry_a_replacement_makes_a_lands_cant_happen() {
    for usurped in [true, false] {
        let (mut game, table) = game();
        if usurped {
            put(&mut game, table, &USURPER, Zone::BATTLEFIELD, P1);
        }
        put(&mut game, table, &LANDER, Zone::BATTLEFIELD, P1);
        let relic = put(&mut game, table, &RELIC, Zone::hand(P0), P0);
        let mut cx = cx(&mut game);
        ops::put_onto_battlefield(&mut cx, &[(relic, P0, false)], ZoneReason::Other).expect("nothing is asked");
        assert_eq!(cx.s().zone(Zone::hand(P0)).contains(&relic), usurped);
        assert_eq!(cx.s().battlefield().iter().any(|&id| cx.s().obj(id).card == table.id_of(&RELIC)), !usurped);
    }
}

/// An entry that can't happen is replaced by no other replacement effect (CR 614.17c): the Priest's "exile it instead" does
/// not apply to a land creature card that can't enter, which stays in the library; it does to a creature card that can.
#[test]
fn no_replacement_effect_replaces_an_entry_that_cant_happen() {
    let (mut game, table) = game();
    put(&mut game, table, &PRIEST, Zone::BATTLEFIELD, P0);
    let arbor = put(&mut game, table, &ARBOR, Zone::library(P0), P0);
    let bear = put(&mut game, table, &BEAR, Zone::library(P0), P0);
    let mut cx = cx(&mut game);
    ops::put_onto_battlefield(&mut cx, &[(arbor, P0, false), (bear, P0, false)], ZoneReason::Other).expect("nothing is asked");
    assert!(cx.s().zone(Zone::library(P0)).contains(&arbor));
    assert!(cx.s().zone(Zone::exile(P0)).iter().any(|&id| cx.s().obj(id).card == table.id_of(&BEAR)));
}

/// Its own "enters as a copy" is judged in turn (CR 614.17d, 616.1c): a land card that would enter as a copy of the Bear can't
/// enter as the land it is first, so its copy replacement never applies (CR 614.17c) and it stays in the library; an artifact
/// card that would enter as a copy of the Waste can until its copy replacement applies, and then can't, so it stays too.
#[test]
fn its_own_copy_replacement_is_judged_in_turn() {
    let (mut game, table) = game();
    let land = put(&mut game, table, &MIMIC_LAND, Zone::library(P0), P0);
    let relic = put(&mut game, table, &MIMIC_RELIC, Zone::library(P0), P0);
    let mut cx = cx(&mut game);
    let mut entering = Entering { object: land, controller: P0, tapped: false, ..ENTERING };
    ops::prepare_entry(&mut cx, &mut entering).expect("nothing is asked");
    assert_eq!(entering.copy, None, "its copy replacement does not apply");
    ops::put_onto_battlefield(&mut cx, &[(land, P0, false), (relic, P0, false)], ZoneReason::Other).expect("nothing is asked");
    assert!(cx.s().zone(Zone::library(P0)).contains(&land));
    assert!(cx.s().zone(Zone::library(P0)).contains(&relic));
}

/// An artifact whose own copy replacement makes it a copy of a land with a copy replacement of its own: once its own has
/// applied it would be a land, so its entry can't happen (CR 614.17d) and no other replacement applies (CR 614.17c) - the
/// land's copy choice is never made, and it stays in the library.
#[test]
fn a_copy_that_would_be_a_land_makes_no_further_copy_choice() {
    let (mut game, table) = game();
    let mimic = put(&mut game, table, &SHIFTING_MIMIC, Zone::library(P0), P0);
    let mut cx = cx(&mut game);
    ops::put_onto_battlefield(&mut cx, &[(mimic, P0, false)], ZoneReason::Other).expect("nothing is asked");
    assert_eq!(SHIFTS.load(std::sync::atomic::Ordering::SeqCst), 0, "the land's copy replacement is not applied");
    assert!(cx.s().zone(Zone::library(P0)).contains(&mimic));
}

const ENTERING: Entering = Entering {
    object: ObjectId::NONE,
    controller: P0,
    tapped: false,
    counters: [0; CounterKind::COUNT],
    copy: None,
    prepared: false,
    chosen_color: None,
    chosen_subtype: None,
    chosen_subtype_pair: None,
    chosen_player: None,
    chosen_card: None,
    chosen_option: None,
    chosen_number: None,
    chosen_object: None,
    emblem: None,
    attach: None,
    attach_player: None,
    x: 0,
    transformed: false,
    protector: None,
    read_ahead: None,
    riot_haste: false,
    instead: None,
    becomes: None,
    own: None,
    melded: None,
    counters_if: None,
};

/// Resolves the permanent spell `spell` on the stack, as its own resolution frame would.
fn resolve_spell(g: &mut Game, def: &'static CardDef, spell: ObjectId) {
    let item = crate::actions::stack_item(StackKind::Spell, spell, AbilityRef::spell(g.cards.id_of(def)), P0, &[]);
    g.state.stack_push(item);
    let serial = g.state.stack.last().expect("the spell").serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    let frame = g.state.agenda.len() - 1;
    let mut cx = Cx { g, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None };
    crate::resolve::resolve(&mut cx).expect("nothing is asked");
}

/// A permanent spell that can't enter as it resolves is put into its owner's graveyard (CR 608.3e): one that would be a land
/// from the start (an artifact spell under "artifacts you control are lands"), and one the Usurper's replacement makes a land
/// (P1's "artifacts you control are lands", under P1) - the stack empty either way.
#[test]
fn a_permanent_spell_that_cant_enter_is_put_into_its_owners_graveyard() {
    for (lander, usurper) in [(P0, false), (P1, true)] {
        let (mut game, table) = game();
        put(&mut game, table, &LANDER, Zone::BATTLEFIELD, lander);
        if usurper {
            put(&mut game, table, &USURPER, Zone::BATTLEFIELD, P1);
        }
        let spell = put(&mut game, table, &RELIC, Zone::STACK, P0);
        resolve_spell(&mut game, &RELIC, spell);
        assert!(game.state.stack.is_empty());
        let gone = game.state.obj(spell).became.expect("it left the stack");
        assert!(game.state.obj(gone).zone.is(ZoneKind::Graveyard), "usurped: {usurper}");
    }
    // And one its own copy replacement makes a land: the example of CR 608.3e, a Clone copying Dryad Arbor.
    let (mut game, table) = game();
    let spell = put(&mut game, table, &MIMIC_RELIC, Zone::STACK, P0);
    resolve_spell(&mut game, &MIMIC_RELIC, spell);
    let gone = game.state.obj(spell).became.expect("it left the stack");
    assert!(game.state.obj(gone).zone.is(ZoneKind::Graveyard));
}
