//! Changes to how spells are paid for that no implemented card makes yet, on a table of test-only cards:
//!
//! - a cost change a resolved spell or ability makes for the turn (CR 611.2a/c, 601.2f, `EffectKind::SpellCostChange`):
//!   Armor Wars's chapter II "Artifact spells you cast this turn cost {1} less to cast", Urza, Planeswalker's +2, Ballad of
//!   the Black Flag's IV, Spellbinding Soprano, Moonrise Towers; Academy Loremaster's "spells they cast this turn cost {2}
//!   more". It outlives its source, changes only its player's spells of its kind, is applied with the static changes
//!   (increases, then reductions), and ends with the turn (CR 514.2).
//! - a static ability that gives the spells a player casts a way of paying besides mana (CR 601.2f-h,
//!   `CardDef::grants_pay_with`): Ironheart, Clever Champion's "Noncreature spells you cast have improvise" (CR 702.126a),
//!   Inspiring Statuary, Chief Engineer's convoke, Teval's delve.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

fn nothing(_: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

const ONE: ManaCost = ManaCost { generic: 1, ..ManaCost::ZERO };

/// "Artifact spells you cast this turn cost {1} less to cast."
fn artifacts_less(q: &dyn Query, _: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    match costed {
        Costed::Spell(spell) if q.chars(spell).is(CardType::Artifact) => CostDelta::less(1),
        _ => CostDelta::NONE,
    }
}

/// "Spells you cast this turn cost {1} more to cast."
fn spells_more(_: &dyn Query, _: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    match costed {
        Costed::Spell(_) => CostDelta::more(1),
        _ => CostDelta::NONE,
    }
}

fn first_change_this_turn(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.spells_cost_this_turn(item, item.controller, 0);
    Ok(())
}

/// An instant for {0}: "Artifact spells you cast this turn cost {1} less to cast."
static WARS: CardDef = CardDef {
    name: "Probe Wars",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], first_change_this_turn),
    lasting_cost_changes: &[LastingCostChange { text: "Artifact spells you cast this turn cost {1} less to cast.", change: artifacts_less, next: None }],
    ..CardDef::BLANK
};

/// "The next noncreature spell you cast this turn costs {1} less to cast."
fn next_noncreature_less(_: &dyn Query, _: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    match costed {
        Costed::Spell(_) => CostDelta::less(1),
        _ => CostDelta::NONE,
    }
}

fn noncreature(q: &dyn Query, spell: ObjectId) -> bool {
    !q.chars(spell).is(CardType::Creature)
}

fn twice_this_turn(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.spells_cost_this_turn(item, item.controller, 0);
    g.end_sentence();
    g.spells_cost_this_turn(item, item.controller, 0);
    Ok(())
}

/// An instant for {0}: "The next noncreature spell you cast this turn costs {1} less to cast."
static NEXT: CardDef = CardDef {
    name: "Probe Next",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], first_change_this_turn),
    lasting_cost_changes: &[LastingCostChange { text: "The next noncreature spell you cast this turn costs {1} less to cast.", change: next_noncreature_less, next: Some(noncreature) }],
    ..CardDef::BLANK
};

/// An instant for {0}: "Spells you cast this turn cost {1} more to cast."
static TAX: CardDef = CardDef {
    name: "Probe Tax",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], first_change_this_turn),
    lasting_cost_changes: &[LastingCostChange { text: "Spells you cast this turn cost {1} more to cast.", change: spells_more, next: None }],
    ..CardDef::BLANK
};

/// "Noncreature spells you cast have improvise."
fn noncreature_improvise(q: &dyn Query, source: ObjectId, caster: PlayerId, spell: ObjectId) -> PayWith {
    if q.controller(source) == caster && !q.chars(spell).is(CardType::Creature) {
        PayWith { improvise: true, ..PayWith::NONE }
    } else {
        PayWith::NONE
    }
}

/// An enchantment for {0}: "Noncreature spells you cast have improvise."
static HEART: CardDef = CardDef { name: "Probe Heart", types: TypeSet::of(&[CardType::Enchantment]), grants_pay_with: &[noncreature_improvise], ..CardDef::BLANK };

/// An artifact for {1}.
static TRINKET: CardDef = CardDef { name: "Probe Trinket", cost: ONE, types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };

/// An artifact for {0}, to be tapped for improvise.
static BAUBLE: CardDef = CardDef { name: "Probe Bauble", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };

/// A sorcery for {1} that does nothing.
static ROCK: CardDef = CardDef { name: "Probe Rock", cost: ONE, types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], nothing), ..CardDef::BLANK };

/// A 1/1 creature for {1}.
static BEAST: CardDef =
    CardDef { name: "Probe Beast", cost: ONE, types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(1), ..CardDef::BLANK };

/// A land with no mana ability: nothing here is paid for with mana.
static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// An instant for {0} that makes the effect of Probe Next twice, as two triggers of one ability resolving in a turn do.
static TWICE: CardDef = CardDef {
    name: "Probe Twice",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], twice_this_turn),
    lasting_cost_changes: &[LastingCostChange { text: "The next noncreature spell you cast this turn costs {1} less to cast.", change: next_noncreature_less, next: Some(noncreature) }],
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&WARS, &TAX, &NEXT, &TWICE, &HEART, &TRINKET, &BAUBLE, &ROCK, &BEAST, &FILLER]))
}

/// A two-player game, Bot1 first, each hand the named cards, then Fillers.
fn deal(first: &[&str], second: &[&str]) -> Game {
    let filler = table().find("Filler").unwrap();
    let deck = |names: &[&str]| {
        let mut deck: Vec<CardId> = names.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect();
        deck.extend(std::iter::repeat(filler).take(30 - deck.len()));
        deck
    };
    let mut game =
        Game::new(table(), &Setup { decks: vec![deck(first), deck(second)], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    game
}

/// Runs the game until Bot1 has priority with an empty stack in its precombat main phase of turn `turn`, everyone else passing,
/// and returns its options.
fn main_phase(game: &mut Game, turn: u32) -> Vec<ActionOption> {
    for _ in 0..400 {
        game.run();
        let Some(request) = game.pending().cloned() else { panic!("the game ended") };
        let answer = match &request.kind {
            RequestKind::Priority { options }
                if request.player == PlayerId(0)
                    && game.state.stack.is_empty()
                    && game.state.turn.number == turn
                    && game.state.turn.step() == Step::PrecombatMain =>
            {
                return options.clone();
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("never reached the main phase of turn {turn}");
}

/// Whether Bot1 is offered the cast of the card named.
fn offered(game: &Game, options: &[ActionOption], name: &str) -> bool {
    options.iter().any(|o| o.kind == ActionKind::CastSpell && o.index == 0 && o.source.is_some_and(|s| game.name(s) == name))
}

/// Bot1 casts the card named from its options, paying with improvise where it is asked to pay, and the spell resolves. The
/// options it has next.
fn cast(game: &mut Game, options: &[ActionOption], name: &str) -> Vec<ActionOption> {
    let option = options
        .iter()
        .find(|o| o.kind == ActionKind::CastSpell && o.index == 0 && o.source.is_some_and(|s| game.name(s) == name))
        .unwrap_or_else(|| panic!("{name} is not offered"))
        .clone();
    let request = game.pending().cloned().unwrap();
    game.answer(request.id, request.player, Answer::Act(Action::of(&option))).unwrap();
    // The payment, if one is asked: tap for improvise while it is offered, then take the pool's way.
    loop {
        game.run();
        let request = game.pending().cloned().unwrap();
        let RequestKind::PayCosts { options, .. } = &request.kind else { break };
        let pick = options
            .iter()
            .find(|o| o.kind == ActionKind::TapToPay && o.index == PAY_IMPROVISE)
            .or_else(|| options.iter().find(|o| o.kind == ActionKind::ConfirmCasting))
            .unwrap_or_else(|| panic!("no way to pay for {name}: {options:?}"))
            .clone();
        game.answer(request.id, request.player, Answer::Act(Action::of(&pick))).unwrap();
    }
    let turn = game.state.turn.number;
    main_phase(game, turn)
}

fn on_battlefield(game: &Game, name: &str) -> Option<ObjectId> {
    game.state.battlefield().iter().copied().find(|&id| game.name(id) == name)
}

fn in_hand(game: &Game, player: PlayerId, name: &str) -> ObjectId {
    game.state.zone(Zone::hand(player)).iter().copied().find(|&id| game.name(id) == name).unwrap_or_else(|| panic!("no {name} in hand"))
}

fn lasting(game: &Game) -> usize {
    game.state.effects.iter().filter(|e| matches!(e.kind, EffectKind::SpellCostChange { .. })).count()
}

/// CR 611.2a/c, 601.2f: "artifact spells you cast this turn cost {1} less" outlives the instant that made it (it is in the
/// graveyard as the artifact is cast), takes the artifact's {1} to {0}, and leaves a sorcery's cost and the other player's
/// artifact alone. It ends with the turn (CR 514.2): on Bot1's next turn the artifact costs {1} again.
#[test]
fn a_reduction_this_turn_outlives_its_source_and_changes_only_its_players_spells_of_its_kind() {
    let mut game = deal(&["Probe Wars", "Probe Trinket", "Probe Trinket", "Probe Rock"], &["Probe Trinket"]);
    let options = main_phase(&mut game, 1);
    // No mana: before the effect nothing that costs {1} can be cast.
    assert!(!offered(&game, &options, "Probe Trinket"));
    assert!(!offered(&game, &options, "Probe Rock"));
    let options = cast(&mut game, &options, "Probe Wars");
    assert!(on_battlefield(&game, "Probe Wars").is_none());
    assert_eq!(lasting(&game), 1);
    assert!(offered(&game, &options, "Probe Trinket"));
    assert!(!offered(&game, &options, "Probe Rock"), "a sorcery is no artifact spell");
    // The menu row says what it costs now; the other player's artifact still costs {1}.
    let trinket = in_hand(&game, PlayerId(0), "Probe Trinket");
    assert_eq!(cast_total(&game, PlayerId(0), &ActionOption::on(ActionKind::CastSpell, trinket, 0)), Some((ONE, ManaCost::ZERO, 0)));
    let theirs = in_hand(&game, PlayerId(1), "Probe Trinket");
    assert_eq!(cast_total(&game, PlayerId(1), &ActionOption::on(ActionKind::CastSpell, theirs, 0)), Some((ONE, ONE, 0)));
    // Every artifact spell this turn, not only the next.
    let options = cast(&mut game, &options, "Probe Trinket");
    assert!(on_battlefield(&game, "Probe Trinket").is_some());
    assert!(offered(&game, &options, "Probe Trinket"));
    // The next turn it is gone.
    let options = main_phase(&mut game, 3);
    assert_eq!(lasting(&game), 0);
    assert!(!offered(&game, &options, "Probe Trinket"));
}

/// CR 601.2f: an increase a resolved effect makes is applied with the reductions, increases first: {1} + {1} is {2}, and less
/// {1} is {1} again.
#[test]
fn an_increase_this_turn_is_applied_before_a_reduction() {
    let mut game = deal(&["Probe Tax", "Probe Wars", "Probe Trinket"], &[]);
    let options = main_phase(&mut game, 1);
    let options = cast(&mut game, &options, "Probe Tax");
    let trinket = in_hand(&game, PlayerId(0), "Probe Trinket");
    assert_eq!(cast_total(&game, PlayerId(0), &ActionOption::on(ActionKind::CastSpell, trinket, 0)), Some((ONE, ManaCost { generic: 2, ..ManaCost::ZERO }, 0)));
    // Probe Wars costs {0} + {1} now, which nothing pays.
    assert!(!offered(&game, &options, "Probe Wars"));
}

/// CR 601.2f-h, 702.126a: "noncreature spells you cast have improvise" lets an untapped artifact pay the {1} of a sorcery, where
/// without it the sorcery can't be cast, and gives a creature spell nothing.
#[test]
fn a_granted_improvise_pays_for_a_noncreature_spell_and_not_a_creature_spell() {
    let mut game = deal(&["Probe Bauble", "Probe Heart", "Probe Rock", "Probe Beast"], &[]);
    let options = main_phase(&mut game, 1);
    let options = cast(&mut game, &options, "Probe Bauble");
    // The control: an untapped artifact, and no improvise yet.
    assert!(!offered(&game, &options, "Probe Rock"));
    let options = cast(&mut game, &options, "Probe Heart");
    assert!(offered(&game, &options, "Probe Rock"));
    assert!(!offered(&game, &options, "Probe Beast"), "a creature spell is not given improvise");
    cast(&mut game, &options, "Probe Rock");
    let bauble = on_battlefield(&game, "Probe Bauble").unwrap();
    assert!(game.state.obj(bauble).tapped, "the artifact paid for it");
    assert!(game.state.zone(Zone::graveyard(PlayerId(0))).iter().any(|&id| game.name(id) == "Probe Rock"));
}

/// CR 611.2a/c, 601.2f: "the next noncreature spell you cast this turn costs {1} less" is not used up by a creature spell
/// cast first, makes the first noncreature spell cost {0}, and is gone for the second, which costs {1}.
#[test]
fn a_next_spell_cost_change_waits_through_a_creature_spell_and_is_used_up_by_the_first_noncreature_spell() {
    let mut game = deal(&["Probe Next", "Probe Beast", "Probe Rock", "Probe Rock"], &[]);
    let options = main_phase(&mut game, 1);
    let options = cast(&mut game, &options, "Probe Next");
    assert_eq!(lasting(&game), 1);
    // Nothing pays a creature spell's {1}, which the effect leaves alone.
    let beast = in_hand(&game, PlayerId(0), "Probe Beast");
    assert_eq!(cast_total(&game, PlayerId(0), &ActionOption::on(ActionKind::CastSpell, beast, 0)), Some((ONE, ONE, 0)));
    let rock = in_hand(&game, PlayerId(0), "Probe Rock");
    assert_eq!(cast_total(&game, PlayerId(0), &ActionOption::on(ActionKind::CastSpell, rock, 0)), Some((ONE, ManaCost::ZERO, 0)));
    assert!(!offered(&game, &options, "Probe Beast"));
    let options = cast(&mut game, &options, "Probe Rock");
    assert_eq!(lasting(&game), 0, "the first noncreature spell used it up");
    assert!(!offered(&game, &options, "Probe Rock"), "the second pays its {{1}}");
}

/// CR 514.2: unused, it ends with the turn.
#[test]
fn a_next_spell_cost_change_not_used_ends_with_the_turn() {
    let mut game = deal(&["Probe Next", "Probe Rock"], &[]);
    let options = main_phase(&mut game, 1);
    cast(&mut game, &options, "Probe Next");
    assert_eq!(lasting(&game), 1);
    let options = main_phase(&mut game, 3);
    assert_eq!(lasting(&game), 0);
    assert!(!offered(&game, &options, "Probe Rock"));
}

/// CR 611.2a, 601.2f: two "next noncreature spell" effects made in one turn both change the first noncreature spell and both
/// are used up by it; the second noncreature spell has neither.
#[test]
fn two_next_spell_cost_changes_are_both_used_up_by_the_same_spell() {
    let mut game = deal(&["Probe Twice", "Probe Rock", "Probe Rock"], &[]);
    let options = main_phase(&mut game, 1);
    let options = cast(&mut game, &options, "Probe Twice");
    assert_eq!(lasting(&game), 2);
    let options = cast(&mut game, &options, "Probe Rock");
    assert_eq!(lasting(&game), 0, "the first noncreature spell used up both");
    assert!(!offered(&game, &options, "Probe Rock"), "the second pays its {{1}}");
}
