//! Plot given to the top card of a library by a rule (`PlayRule::LibraryTopPlot`, CR 702.170a, 116.2k): a special action of
//! the player's own, offered for the nonland card on top of their library at sorcery timing, for that card's mana cost, which
//! exiles the card and makes it plotted. The cards here are small probes for the rule, not card implementations.

use std::sync::OnceLock;

use cantrip_api::*;
use cantrip_rules::{describe, Game as RulesGame, Setup};
use cantrip_state::{Frame, FrameKind};

const ME: PlayerId = PlayerId(0);
const OPPONENT: PlayerId = PlayerId(1);

static SPELL: CardDef = CardDef {
    name: "Plot Probe Spell",
    oracle: true,
    cost: ManaCost { generic: 2, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: Some(SpellDef::MODAL),
    ..CardDef::BLANK
};

static X_SPELL: CardDef = CardDef {
    name: "Plot Probe X Spell",
    cost: ManaCost { generic: 1, x: 1, ..ManaCost::ZERO },
    ..SPELL
};

fn plot_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.exile(item.object);
    g.become_plotted(exiled);
    Ok(())
}

/// A card with plot of its own, "Plot {1}", for a mana cost of {2}: the ruling on Fblthp says it may use either cost.
static PLOTTER: CardDef = CardDef {
    name: "Plot Probe Plotter",
    abilities: &[ability(Cost::mana(ManaCost { generic: 1, ..ManaCost::ZERO }), plot_it, "Plot {1}").sorcery_speed().from(ZoneKind::Hand).plot()],
    ..SPELL
};

/// The same, with a plot cost equal to its mana cost: the one cost, offered once.
static SAME_PLOTTER: CardDef = CardDef {
    name: "Plot Probe Same Plotter",
    abilities: &[ability(Cost::mana(ManaCost { generic: 2, ..ManaCost::ZERO }), plot_it, "Plot {2}").sorcery_speed().from(ZoneKind::Hand).plot()],
    ..SPELL
};

static LAND: CardDef = CardDef { name: "Plot Probe Land", oracle: true, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

static SOURCE: CardDef = CardDef {
    name: "Plot Probe Source",
    oracle: true,
    types: TypeSet::of(&[CardType::Artifact]),
    play_rules: &[PlayRule::LibraryTopPlot],
    ..CardDef::BLANK
};

static FILLER: CardDef = CardDef { name: "Plot Probe Filler", oracle: true, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn nothing(_: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// A permanent whose effect lets a player take an action at a later time (CR 116.2c), "Probe action {1}".
static ACTOR: CardDef = CardDef {
    name: "Plot Probe Actor",
    oracle: true,
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost::mana(ManaCost { generic: 1, ..ManaCost::ZERO }), nothing, "Probe action {1}")],
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&SPELL, &X_SPELL, &PLOTTER, &SAME_PLOTTER, &LAND, &SOURCE, &FILLER, &ACTOR]))
}

/// A game in the first player's precombat main phase with priority, `top` on top of their library, `pool` colorless mana in
/// their pool, and the rule's source on their battlefield when `source` is some player's.
fn game(top: &str, pool: u8, source: Option<PlayerId>) -> (RulesGame, ObjectId) {
    game_at(top, pool, source, Step::PrecombatMain)
}

fn game_at(top: &str, pool: u8, source: Option<PlayerId>, step: Step) -> (RulesGame, ObjectId) {
    game_setup(top, pool, source, step, |_| {})
}

/// [`game_at`], with `setup` run on the game before the first player is asked for an action.
fn game_setup(top: &str, pool: u8, source: Option<PlayerId>, step: Step, setup: impl FnOnce(&mut RulesGame)) -> (RulesGame, ObjectId) {
    let cards = table();
    let (top_card, filler) = (cards.find(top).unwrap(), cards.find("Plot Probe Filler").unwrap());
    let mut game = RulesGame::new(cards, &Setup { decks: vec![vec![top_card, filler, filler, filler, filler, filler, filler], vec![filler; 7]], seed: 1, ..Setup::default() });
    let id = game.state.zone(Zone::hand(ME))[0];
    let from = game.state.obj(id).zone;
    assert!(game.state.zone_remove(from, id));
    game.state.update_obj(id, |o| o.zone = Zone::library(ME));
    game.state.zone_insert(Zone::library(ME), 0, id);
    if let Some(player) = source {
        let card = cards.find("Plot Probe Source").unwrap();
        let permanent = game.state.push_obj(Obj::new(card, player, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, permanent);
    }
    game.state.update_turn(|t| {
        t.number = 1;
        t.active = ME;
        t.priority = ME;
        t.set_step(step);
        t.began = true;
    });
    if pool > 0 {
        game.state.add_mana(ME, Mana::C, u32::from(pool), ObjectId::NONE, None, ManaLasts::Step, false);
    }
    setup(&mut game);
    game.state.frame_push(Frame::new(FrameKind::Priority));
    game.run();
    (game, id)
}

fn plots(game: &RulesGame) -> Vec<ActionOption> {
    match &game.pending().expect("a priority request").kind {
        RequestKind::Priority { options } => options.iter().filter(|o| o.kind == ActionKind::ActivateAbility && o.index >= PLAYER_SPECIAL_ACTION).cloned().collect(),
        other => panic!("expected priority, got {other:?}"),
    }
}

fn take(game: &mut RulesGame, option: &ActionOption) {
    let request = game.pending().unwrap().clone();
    game.answer(request.id, request.player, Answer::Act(Action::of(option))).unwrap();
    game.run();
    while let Some(RequestKind::PayCosts { options, .. }) = game.pending().map(|r| r.kind.clone()) {
        let confirm = options.iter().find(|o| o.kind == ActionKind::AssignMana).or_else(|| options.iter().find(|o| o.kind == ActionKind::ConfirmCasting)).expect("a way to pay");
        let request = game.pending().unwrap().clone();
        game.answer(request.id, request.player, Answer::Act(Action::of(confirm))).unwrap();
        game.run();
    }
}

fn colorless(game: &RulesGame) -> u32 {
    game.state.player(ME).pool.0[Mana::C as usize]
}

/// CR 702.170a, 116.2k: the nonland card on top of the library may be plotted for its mana cost: it is exiled, and becomes
/// plotted, so it may be cast on a later turn without paying its mana cost.
#[test]
fn the_top_card_is_plotted_for_its_mana_cost() {
    let (mut game, top) = game("Plot Probe Spell", 2, Some(ME));
    let [option] = plots(&game).try_into().expect("one plot action");
    assert_eq!(option.source, Some(top), "of the top card");
    take(&mut game, &option);
    assert_eq!(colorless(&game), 0, "its mana cost {{2}} paid");
    assert!(!game.state.zone(Zone::library(ME)).contains(&top), "no longer in the library");
    let plotted = game.log.iter().find_map(|r| match r.event {
        Event::Plotted { card, player: ME } => Some(card),
        _ => None,
    });
    let card = plotted.expect("it became plotted");
    assert!(game.state.zone(Zone::exile(ME)).contains(&card), "exiled");
    assert_eq!(game.state.obj(card).card, game.state.obj(top).card, "the card plotted is the top card");
}

/// "Nonland cards": a land on top is not offered.
#[test]
fn a_land_on_top_is_not_plotted() {
    let (game, _) = game("Plot Probe Land", 4, Some(ME));
    assert!(plots(&game).is_empty());
}

/// Without the rule, or with the opponent's rule (it is "your library"), nothing is offered.
#[test]
fn only_the_rules_controller_plots_from_their_library() {
    assert!(plots(&game("Plot Probe Spell", 4, None).0).is_empty(), "no rule");
    assert!(plots(&game("Plot Probe Spell", 4, Some(OPPONENT)).0).is_empty(), "the opponent's rule");
}

/// The cost must be payable: {2} with one mana is not offered.
#[test]
fn a_plot_cost_that_cannot_be_paid_is_not_offered() {
    assert!(plots(&game("Plot Probe Spell", 1, Some(ME)).0).is_empty());
}

/// CR 107.3g-h: an X in the mana cost is 0 when the card is plotted: {X}{1} costs {1}.
#[test]
fn x_is_zero_in_a_plot_cost() {
    let (mut game, _) = game("Plot Probe X Spell", 1, Some(ME));
    let [option] = plots(&game).try_into().expect("one plot action");
    take(&mut game, &option);
    assert_eq!(colorless(&game), 0);
}

/// CR 116.2k, 702.170a: sorcery timing only - a special action "any time you have priority during your main phase while the
/// stack is empty": not offered in the beginning of combat step.
#[test]
fn it_is_taken_at_sorcery_timing_only() {
    assert_eq!(plots(&game_at("Plot Probe Spell", 2, Some(ME), Step::PrecombatMain).0).len(), 1);
    assert!(plots(&game_at("Plot Probe Spell", 2, Some(ME), Step::BeginCombat).0).is_empty());
}

/// A card with plot itself, on top: it may be plotted for the rule's cost (its mana cost, {2}) or for its own plot cost ({1}),
/// each an action of its own; a plot cost the same as its mana cost is no second action.
#[test]
fn a_card_with_plot_may_use_its_own_cost_or_the_rules() {
    let (g, top) = game("Plot Probe Plotter", 2, Some(ME));
    let options = plots(&g);
    assert_eq!(options.len(), 2, "the rule's plot and the card's own");
    assert!(options.iter().all(|o| o.source == Some(top)));
    assert_ne!(describe(g.cards, &g.state, &g.log, &options[0]), describe(g.cards, &g.state, &g.log, &options[1]), "told apart");
    assert_eq!(describe(g.cards, &g.state, &g.log, &options[1]), "Plot {1}");
    let (mut g, _) = game("Plot Probe Plotter", 2, Some(ME));
    let options = plots(&g);
    take(&mut g, &options[1]);
    assert_eq!(colorless(&g), 1, "its own plot cost {{1}} paid");
    assert!(g.log.iter().any(|r| matches!(r.event, Event::Plotted { player: ME, .. })));
    let (mut g, _) = game("Plot Probe Plotter", 2, Some(ME));
    let options = plots(&g);
    take(&mut g, &options[0]);
    assert_eq!(colorless(&g), 0, "its mana cost {{2}} paid");
    assert_eq!(plots(&game("Plot Probe Same Plotter", 2, Some(ME)).0).len(), 1, "one cost, one action");
    assert_eq!(plots(&game("Plot Probe Plotter", 1, Some(ME)).0).len(), 1, "only the cheaper, where one mana is all there is");
}

/// An effect of `controller`'s that lets them take the action "Probe action {1}" later, from a permanent of the same player's:
/// that permanent.
fn let_act(game: &mut RulesGame, controller: PlayerId, timestamp: u32) -> ObjectId {
    let card = table().find("Plot Probe Actor").unwrap();
    let actor = game.state.push_obj(Obj::new(card, controller, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, actor);
    game.state.effect_push(ContinuousEffect {
        source: actor,
        controller,
        timestamp,
        affected: actor,
        kind: EffectKind::PlayerSpecialAction { ability: AbilityRef { card, index: 0 }, about: Target::None },
        duration: Duration::Indefinite,
        words: Words::NONE,
    });
    actor
}

/// CR 116.2c: the plot actions follow the actions the player's own effects let them take, and only theirs: with one such effect
/// of mine and two of the opponent's, the options are mine, then the rule's plot ({2}), then the card's own ({1}), each told
/// by its own words.
#[test]
fn plots_follow_the_actions_of_the_players_own_effects_and_no_others() {
    let mut mine = ObjectId::NONE;
    let (g, top) = game_setup("Plot Probe Plotter", 5, Some(ME), Step::PrecombatMain, |g| {
        let_act(g, OPPONENT, 1);
        mine = let_act(g, ME, 2);
        let_act(g, OPPONENT, 3);
    });
    let options = plots(&g);
    assert_eq!(options.len(), 3, "my effect's action, the rule's plot and the card's own");
    assert_eq!(options.iter().map(|o| o.source).collect::<Vec<_>>(), [Some(mine), Some(top), Some(top)]);
    let text = |o: &ActionOption| describe(g.cards, &g.state, &g.log, o);
    assert_eq!(text(&options[0]), "Probe action {1}");
    assert_eq!(text(&options[2]), "Plot {1}", "the card's own plot is told as its own");
    assert_ne!(text(&options[1]), "Plot {1}", "and the rule's is not");
    assert_ne!(text(&options[1]), text(&options[0]));
}
