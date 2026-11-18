//! A spell's own "exile it instead" as it leaves the stack (CR 614.1a): flashback's "exile this card instead of putting it
//! anywhere else any time it would leave the stack" (CR 702.34a), and the "if that spell would be put into your graveyard,
//! exile it instead" an effect that has a spell cast makes (`ExileInstead`) - each exactly as worded: no type condition
//! unless the text has one, no destination but the one it names, and ordered by the spell's controller with the other
//! replacement effects that would change the same move (CR 616.1). The cards here are probes, not card implementations.

use std::sync::OnceLock;

use cantrip_api::*;
use cantrip_rules::{Game as RulesGame, Setup};
use cantrip_state::{Frame, FrameKind};

const ME: PlayerId = PlayerId(0);

static INSTANT: CardDef = CardDef {
    name: "Leave Instant",
    oracle: true,
    cost: ManaCost { generic: 1, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    spell: Some(SpellDef::MODAL),
    ..CardDef::BLANK
};

/// A permanent card an effect may cast from a graveyard, as Dreadhorde Arcanist may the creature face of a card, or Bilbo an
/// artifact.
static ARTIFACT: CardDef = CardDef { name: "Leave Artifact", types: TypeSet::of(&[CardType::Artifact]), spell: None, ..INSTANT };

static FLASHBACK_WAY: WayDef = cantrip_api::way(Cost::mana(ManaCost::ZERO), ZoneKind::Graveyard, "Flashback {0}").exile_after();

static FLASHBACK_INSTANT: CardDef = CardDef { name: "Leave Flashback Instant", ways: &[FLASHBACK_WAY], ..INSTANT };

/// The first card of its controller's graveyard, cast for nothing on the terms `exile_instead` says, then sent off the stack
/// as `then` says (`None`: left to resolve).
fn cast_then(g: &mut dyn cantrip_api::Game, item: &StackItem, exile_instead: ExileInstead, then: Option<CounterTo>, counter: bool) -> Result<(), Suspend> {
    let Some(&card) = g.state().zone(Zone::graveyard(item.controller)).first() else { return Ok(()) };
    g.cast_during_resolution_as(card, CastTerms { instead: Some(ManaCost::ZERO), exile_instead, ..CastTerms::default() })?;
    let Some(spell) = g.state().stack.last().map(|i| i.object).filter(|&s| s != item.object) else { return Ok(()) };
    if counter {
        g.counter(spell);
    } else if let Some(to) = then {
        g.remove_spell(spell, to);
    }
    Ok(())
}

fn counter_any(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cast_then(g, item, ExileInstead::Spell, None, true)
}

fn counter_instant_or_sorcery(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cast_then(g, item, ExileInstead::InstantOrSorcery, None, true)
}

fn bounce_any(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cast_then(g, item, ExileInstead::Spell, Some(CounterTo::Hand), false)
}

fn bottom_any(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cast_then(g, item, ExileInstead::Spell, Some(CounterTo::LibraryBottom), false)
}

/// The first card of its controller's graveyard cast for nothing as it is, then countered: a flashback card by its flashback.
fn counter_by_way(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(&card) = g.state().zone(Zone::graveyard(item.controller)).first() else { return Ok(()) };
    g.cast_during_resolution_as(card, CastTerms::default())?;
    if let Some(spell) = g.state().stack.last().map(|i| i.object).filter(|&s| s != item.object) {
        g.counter(spell);
    }
    Ok(())
}

/// The first card of its controller's graveyard cast for nothing, left to resolve, with "exile that spell instead of putting it into
/// your graveyard as it resolves. If you do, it becomes plotted" (Lilah, Undefeated Slickshot's).
fn cast_and_plot(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cast_then(g, item, ExileInstead::Plot, None, false)
}

macro_rules! stager {
    ($name:literal, $effect:ident) => {
        CardDef { name: $name, cost: ManaCost::ZERO, spell: Some(SpellDef { targets: &[], effect: $effect }), ..INSTANT }
    };
}

static COUNTER_ANY: CardDef = stager!("Leave Counter Any", counter_any);
static CAST_AND_PLOT: CardDef = stager!("Leave Cast And Plot", cast_and_plot);
static COUNTER_INSTANT_OR_SORCERY: CardDef = stager!("Leave Counter Instant Or Sorcery", counter_instant_or_sorcery);
static BOUNCE_ANY: CardDef = stager!("Leave Bounce Any", bounce_any);
static BOTTOM_ANY: CardDef = stager!("Leave Bottom Any", bottom_any);
static COUNTER_BY_WAY: CardDef = stager!("Leave Counter By Way", counter_by_way);

/// "If a card would be put into a graveyard from anywhere, put it on the bottom of its owner's library instead" (Wheel of
/// Sun and Moon's kind).
fn to_a_graveyard(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.is_card && z.to.is(ZoneKind::Graveyard))
}

fn to_its_library(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.to = Zone::library(z.to.player);
    }
}

static RECYCLER: CardDef = CardDef {
    name: "Leave Recycler",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, to_a_graveyard, to_its_library, "put it into its library instead")],
    ..CardDef::BLANK
};

static FILLER: CardDef = CardDef { name: "Leave Filler", has_cost: false, ..CardDef::BLANK };

/// The first card of its controller's graveyard cast by its flashback, then returned to its owner's hand by an instruction
/// that is no counterspell's (a move of the spell like any other's).
fn return_by_way(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(&card) = g.state().zone(Zone::graveyard(item.controller)).first() else { return Ok(()) };
    g.cast_during_resolution_as(card, CastTerms::default())?;
    if let Some(spell) = g.state().stack.last().map(|i| i.object).filter(|&s| s != item.object) {
        g.return_to_hand(&[spell]);
    }
    Ok(())
}

static RETURN_BY_WAY: CardDef = stager!("Leave Return By Way", return_by_way);

/// A modal double-faced card (CR 712.2) whose front is an instant and whose back a creature with a cost of its own.
static CREATURE_BACK: CardDef = CardDef {
    name: "Leave Creature Back",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    spell: None,
    ..INSTANT
};
static MODAL: CardDef = CardDef { name: "Leave Modal Instant", back: Some(&CREATURE_BACK), ..INSTANT };

fn nothing(_: &dyn Query, _: &Asker, _: ObjectId, _: PlayerId) -> Option<Cost> {
    Some(Cost::NONE)
}

/// "Each card in your graveyard has flashback" for nothing (Iroh's kind, CR 702.34a), the way a permanent gives.
static FLASHBACK_GIVER: CardDef = CardDef {
    name: "Leave Flashback Giver",
    types: TypeSet::of(&[CardType::Artifact]),
    granted_ways: &[GrantedWay { text: "free flashback", cost: nothing, flashback: true, ..GrantedWay::CAST }],
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        CardTable::new([
            &INSTANT,
            &ARTIFACT,
            &FLASHBACK_INSTANT,
            &COUNTER_ANY,
            &COUNTER_INSTANT_OR_SORCERY,
            &BOUNCE_ANY,
            &BOTTOM_ANY,
            &COUNTER_BY_WAY,
            &RECYCLER,
            &FILLER,
            &RETURN_BY_WAY,
            &CREATURE_BACK,
            &MODAL,
            &FLASHBACK_GIVER,
            &CAST_AND_PLOT,
        ])
    })
}

/// A game in the first player's precombat main phase with priority: `stager` in their hand, `card` the only card of their
/// graveyard, and `Leave Recycler` on the battlefield if `recycler`: the game, the stager and the recycler.
fn setup(stager: &str, card: &str, recycler: bool) -> (RulesGame, ObjectId, Option<ObjectId>) {
    let cards = table();
    let (stager, card, filler) = (cards.find(stager).unwrap(), cards.find(card).unwrap(), cards.find("Leave Filler").unwrap());
    let mut game = RulesGame::new(cards, &Setup { decks: vec![vec![stager, card, filler, filler, filler, filler, filler], vec![filler; 7]], seed: 1, ..Setup::default() });
    let (stager_id, card_id) = (game.state.zone(Zone::hand(ME))[0], game.state.zone(Zone::hand(ME))[1]);
    assert!(game.state.zone_remove(Zone::hand(ME), card_id));
    game.state.update_obj(card_id, |o| o.zone = Zone::graveyard(ME));
    game.state.zone_push(Zone::graveyard(ME), card_id);
    let recycler = recycler.then(|| {
        let id = game.state.push_obj(Obj::new(cards.find("Leave Recycler").unwrap(), ME, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        id
    });
    game.state.update_turn(|t| {
        t.number = 1;
        t.active = ME;
        t.priority = ME;
        t.set_step(Step::PrecombatMain);
        t.began = true;
    });
    game.state.frame_push(Frame::new(FrameKind::Priority));
    (game, stager_id, recycler)
}

/// Casts the stager and plays on until the stack is empty, confirming payments and passing. Each order of replacement effects
/// asked is answered by `order`, given the sources offered; what was asked is returned.
fn play(game: &mut RulesGame, stager: ObjectId, order: impl Fn(&[ObjectId]) -> usize) -> Vec<Vec<ObjectId>> {
    game.run();
    let cast = match &game.pending().expect("priority").kind {
        RequestKind::Priority { options } => options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(stager)).cloned().expect("the stager"),
        other => panic!("expected priority, got {other:?}"),
    };
    let request = game.pending().unwrap().clone();
    game.answer(request.id, request.player, Answer::Act(Action::of(&cast))).unwrap();
    let mut asked = Vec::new();
    for _ in 0..32 {
        game.run();
        let cast = game.log.iter().any(|r| matches!(r.event, Event::SpellCast { player: ME, .. }));
        if game.state.stack.is_empty() && cast {
            return asked;
        }
        let request = game.pending().cloned().expect("asked before the stack is empty");
        let answer = match &request.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::PayCosts { options, .. } => {
                let confirm = options
                    .iter()
                    .find(|o| o.kind == ActionKind::AssignMana)
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::ConfirmCasting))
                    .unwrap_or_else(|| panic!("unexpected payment options: {options:?}"));
                Answer::Act(Action::of(confirm))
            }
            // The cost: flashback's, where it is offered.
            RequestKind::ChooseModes { modes, eligible, .. } => {
                let way = eligible.iter().copied().find(|&i| matches!(modes[i as usize], Label::Way(..)));
                Answer::Modes(vec![way.unwrap_or(eligible[0])])
            }
            RequestKind::ChooseReplacement { sources, .. } => {
                assert_eq!(request.player, ME, "the spell's controller orders them (CR 616.1)");
                asked.push(sources.clone());
                Answer::Index(order(sources) as u8)
            }
            other => panic!("unexpected request: {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the stack did not empty");
}

/// The zone the one card named `card` is in: its owner's graveyard, exile, hand or library, or the battlefield.
fn where_is(game: &RulesGame, card: &str) -> ZoneKind {
    let card = table().find(card).unwrap();
    let zones = [Zone::graveyard(ME), Zone::exile(ME), Zone::hand(ME), Zone::library(ME), Zone::BATTLEFIELD];
    zones.into_iter().find(|&zone| game.state.zone(zone).iter().any(|&id| game.state.obj(id).card == card)).expect("somewhere").kind()
}

/// "If that spell would be put into your graveyard, exile it instead" names no type: a permanent spell cast this way that is
/// countered is exiled (CR 614.1a) - the creature face Dreadhorde Arcanist may cast, an artifact Jace's permission may.
#[test]
fn a_spell_of_any_type_cast_with_exile_instead_and_countered_is_exiled() {
    let (mut game, stager, _) = setup("Leave Counter Any", "Leave Artifact", false);
    play(&mut game, stager, |_| 0);
    assert_eq!(where_is(&game, "Leave Artifact"), ZoneKind::Exile);
}

/// "If an instant or sorcery spell cast this way would be put into your graveyard" (Bilbo's) does name types: an artifact
/// spell countered goes to the graveyard, an instant is exiled.
#[test]
fn only_an_instant_or_sorcery_is_exiled_where_the_text_says_so() {
    let (mut game, stager, _) = setup("Leave Counter Instant Or Sorcery", "Leave Artifact", false);
    play(&mut game, stager, |_| 0);
    assert_eq!(where_is(&game, "Leave Artifact"), ZoneKind::Graveyard);
    let (mut game, stager, _) = setup("Leave Counter Instant Or Sorcery", "Leave Instant", false);
    play(&mut game, stager, |_| 0);
    assert_eq!(where_is(&game, "Leave Instant"), ZoneKind::Exile);
}

/// It replaces the move to the graveyard alone: a spell returned to its owner's hand, or put into a library, goes there (Jace,
/// Telepath Unbound's rulings), where flashback's "anywhere else" exiles it (CR 702.34a).
#[test]
fn exile_instead_of_the_graveyard_leaves_a_return_to_hand_or_library_alone() {
    let (mut game, stager, _) = setup("Leave Bounce Any", "Leave Instant", false);
    play(&mut game, stager, |_| 0);
    assert_eq!(where_is(&game, "Leave Instant"), ZoneKind::Hand);
    let (mut game, stager, _) = setup("Leave Bottom Any", "Leave Instant", false);
    play(&mut game, stager, |_| 0);
    assert_eq!(where_is(&game, "Leave Instant"), ZoneKind::Library);
    assert_eq!(game.state.zone(Zone::library(ME)).first().map(|&id| game.state.obj(id).card), table().find("Leave Instant"), "on the bottom");
}

/// CR 616.1: the spell's own "exile it instead" is ordered with the other replacement effects that would change its move to
/// the graveyard, by the spell's controller. Taken first, it is exiled; the other first, it goes to its library, and the
/// spell's own no longer applies (CR 616.1f).
#[test]
fn exile_instead_is_ordered_with_the_other_replacements_by_the_spells_controller() {
    for own_first in [true, false] {
        let (mut game, stager, recycler) = setup("Leave Counter Any", "Leave Instant", true);
        let asked = play(&mut game, stager, |sources| sources.iter().position(|&s| (Some(s) != recycler) == own_first).unwrap());
        assert_eq!(asked.len(), 1, "asked once: {asked:?}");
        assert_eq!(asked[0].len(), 2);
        let expected = if own_first { ZoneKind::Exile } else { ZoneKind::Library };
        assert_eq!(where_is(&game, "Leave Instant"), expected, "own first: {own_first}");
    }
}

/// Flashback's is ordered with them too (CR 616.1), and as it replaces a move anywhere but exile, it applies again once the
/// other has sent the card to its library (CR 616.1f): exiled either way.
#[test]
fn flashback_is_ordered_with_the_other_replacements_and_applies_after_them() {
    for own_first in [true, false] {
        let (mut game, stager, recycler) = setup("Leave Counter By Way", "Leave Flashback Instant", true);
        let asked = play(&mut game, stager, |sources| sources.iter().position(|&s| (Some(s) != recycler) == own_first).unwrap());
        assert_eq!(asked.len(), 1, "asked once: {asked:?}");
        assert_eq!(where_is(&game, "Leave Flashback Instant"), ZoneKind::Exile, "own first: {own_first}");
    }
}

/// CR 702.34a: flashback's replacement is the spell's own, of any move off the stack - one an instruction makes with no word
/// of countering (a return to its owner's hand) as much as resolving or being countered: the move carries it, read off the
/// spell, whatever made it.
#[test]
fn a_flashback_spell_returned_to_hand_by_any_instruction_is_exiled() {
    let (mut game, stager, _) = setup("Leave Return By Way", "Leave Flashback Instant", false);
    play(&mut game, stager, |_| 0);
    assert_eq!(where_is(&game, "Leave Flashback Instant"), ZoneKind::Exile);
}

/// The casts a priority request offers of `card` (the way of each, and what offered it).
fn casts(game: &mut RulesGame, card: ObjectId) -> Vec<(u8, Option<ObjectId>)> {
    game.run();
    match &game.pending().expect("priority").kind {
        RequestKind::Priority { options } => options
            .iter()
            .filter(|o| o.kind == ActionKind::CastSpell && o.source == Some(card))
            .map(|o| (o.index, o.granted_by.map(|g| g.object)))
            .collect(),
        other => panic!("expected priority, got {other:?}"),
    }
}

/// CR 702.34a: "You may cast this card from your graveyard if the resulting spell is an instant or sorcery spell": flashback a
/// permission gives (Snapcaster Mage's) or a permanent gives (Iroh's) casts the instant front of a modal double-faced card, never
/// its creature back - which a permission that is no flashback does cast (CR 712.8a).
#[test]
fn granted_flashback_casts_only_an_instant_or_sorcery_face() {
    let (mut game, _, _) = setup("Leave Filler", "Leave Modal Instant", false);
    let card = game.state.zone(Zone::graveyard(ME))[0];
    let source = game.state.push_obj(Obj::new(table().find("Leave Filler").unwrap(), ME, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, source);
    let permission = |grant| PlayPermission { what: Playable::Card(card), player: ME, grant, by: Grant { object: source, ability: AbilityRef::spell(CardId(0)) }, turn: 1 };
    game.state.update_player(ME, |p| p.pool.0[Mana::C as usize] = 2);

    let mut plain = game.clone();
    plain.state.permission_push(permission(PlayGrant::CAST));
    let offered = casts(&mut plain, card);
    assert!(offered.contains(&(0, Some(source))) && offered.contains(&(WAY_BACK_FACE, Some(source))), "{offered:?}");

    let mut flashback = game.clone();
    flashback.state.permission_push(permission(PlayGrant { flashback: true, ..PlayGrant::CAST }));
    let offered = casts(&mut flashback, card);
    assert!(offered.contains(&(0, Some(source))), "{offered:?}");
    assert!(!offered.iter().any(|&(way, _)| way == WAY_BACK_FACE), "{offered:?}");

    let mut given = game.clone();
    let giver = given.state.push_obj(Obj::new(table().find("Leave Flashback Giver").unwrap(), ME, Zone::BATTLEFIELD));
    given.state.zone_push(Zone::BATTLEFIELD, giver);
    given.state.see_statics();
    let offered = casts(&mut given, card);
    assert!(offered.contains(&(0, Some(giver))), "{offered:?}");
    assert!(!offered.iter().any(|&(way, _)| way == WAY_BACK_FACE), "{offered:?}");
}

/// A spell Lilah has exiled as it resolves becomes plotted the one way every card does (`Game::become_plotted`, CR 702.170a): its
/// owner may cast it for free on a later turn, and it is a plotted event, naming the card in exile, that "when this card becomes
/// plotted" sees.
#[test]
fn a_spell_exiled_and_plotted_as_it_resolves_is_a_plotted_event_for_its_owner() {
    let (mut game, stager, _) = setup("Leave Cast And Plot", "Leave Instant", false);
    play(&mut game, stager, |_| 0);
    assert_eq!(where_is(&game, "Leave Instant"), ZoneKind::Exile);
    let exiled = game.state.zone(Zone::exile(ME))[0];
    let plotted: Vec<_> = game.log.iter().filter_map(|r| if let Event::Plotted { card, player } = r.event { Some((card, player)) } else { None }).collect();
    assert_eq!(plotted, vec![(exiled, ME)]);
}
