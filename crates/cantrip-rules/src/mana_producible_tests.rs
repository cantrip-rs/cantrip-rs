//! CR 106.7: capabilities are read without paying costs or changing the game. Dependencies with no independently
//! defined mana give none (CR 106.5); the same dependency network gains only the types supplied by an independent land.

use crate::{Game, Setup};
use cantrip_api::*;

fn in_frame<T>(g: &mut Game, kind: cantrip_state::FrameKind, answers: Vec<Answer>, f: impl FnOnce(&mut crate::Cx<'_>) -> T) -> T {
    g.state.frame_push(cantrip_state::Frame::new(kind));
    let frame = g.state.agenda.len() - 1;
    g.state.update_frame(frame, |f| f.answers = answers.into_iter().map(|a| (None, a)).collect());
    f(&mut crate::Cx { g, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None })
}

fn assert_capability_isolated(g: &mut Game, source: ObjectId, expected: (ColorSet, bool)) {
    fn snapshot(g: &Game) -> String {
        let mut state = g.state.clone();
        // A layers read may populate this derived cache; it is not game state or history.
        state.static_sources = std::cell::RefCell::new(cantrip_state::StaticSources::none());
        format!("{state:?}")
    }
    let state = snapshot(g);
    let history = format!("{:?}", g.log);
    let at = g.at;
    assert_eq!(g.producible_types(source), expected, "Game query of {}", g.chars(source).name);
    let reader = crate::resolve::Reader { cards: g.cards, state: &g.state, events: &g.log };
    assert_eq!(reader.producible_types(source), expected, "raw-state Reader query");
    let cx = crate::Cx { g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None };
    assert_eq!(cx.producible_types(source), expected, "in-flight Cx query");
    assert!(snapshot(cx.g) == state, "every state field, including stack, zones and mana, is unchanged");
    assert_eq!(format!("{:?}", cx.g.log), history, "history is unchanged");
    assert_eq!(cx.g.at, at, "game-log tree position is unchanged");
    assert!(cx.g.producible_path.borrow().is_empty());
}

fn grave_land(q: &dyn Query, _: &Asker, target: Target) -> bool {
    cantrip_mechanics::targets::card_in_graveyard(q, target, &[CardType::Land])
}

// The target determines the possible colors; a later choice and its zone change must also be hypothetical.
fn targeted_mana(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if g.target_legal(item, 0) {
        let id = g.state().target(item, 0).object().unwrap();
        let choices = g.chars(id).colors;
        if cantrip_mechanics::exile_target(g, item, 0) {
            let color = g.ask(item.controller, RequestKind::ChooseColors {
                source: item.object, choices, count: 1, purpose: ColorUse::AddMana,
            })?;
            if let Answer::Colors(colors) = color {
                g.add_mana(item.controller, colors[0].mana(), 1);
            }
        }
    }
    Ok(())
}

static TARGETED: CardDef = CardDef {
    name: "Capability targeted stack land",
    abilities: &[ability(cost("{99}, {T}"), targeted_mana, "Exile target land card. Add one mana of one of its colors.")
        .targets(&[target(grave_land)]).produces_mana().condition(|_, _| false)],
    ..FILLER
};
static GREEN_TARGET: CardDef = CardDef { name: "Green graveyard land", colors: Some(ColorSet::of(Color::Green)), ..FILLER };
static RED_TARGET: CardDef = CardDef { name: "Red graveyard land", colors: Some(ColorSet::of(Color::Red)), ..FILLER };
static INVALID_TARGET: CardDef = CardDef { name: "Invalid blue graveyard creature", types: TypeSet::of(&[CardType::Creature]),
    colors: Some(ColorSet::of(Color::Blue)), ..CardDef::BLANK };

fn add_object(g: &mut Game, def: &'static CardDef, player: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), player, zone));
    g.state.zone_push(zone, id);
    id
}

fn stack_game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([
        &FILLER, &TARGETED, &GREEN_TARGET, &RED_TARGET, &INVALID_TARGET, &CONTAMINATION, &ORCHARD,
    ])));
    Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&FILLER); 20]; 2], ..Default::default() })
}

#[test]
fn stack_capability_enumerates_legal_targets_and_effect_choices_without_paying_or_mutating() {
    let mut g = stack_game();
    let source = put(&mut g, &TARGETED, 1);
    g.state.update_obj(source, |o| o.tapped = true);
    assert_capability_isolated(&mut g, source, (ColorSet::NONE, false));
    add_object(&mut g, &INVALID_TARGET, PlayerId(0), Zone::graveyard(PlayerId(0)));
    assert_capability_isolated(&mut g, source, (ColorSet::NONE, false));
    add_object(&mut g, &GREEN_TARGET, PlayerId(1), Zone::graveyard(PlayerId(1)));
    let red = add_object(&mut g, &RED_TARGET, PlayerId(0), Zone::graveyard(PlayerId(0)));
    let colors = ColorSet(ColorSet::of(Color::Green).0 | ColorSet::of(Color::Red).0);
    assert_capability_isolated(&mut g, source, (colors, false));
    // Recursion sees the target-backed production without exiling its witnesses.
    let orchard = put(&mut g, &ORCHARD, 0);
    assert_capability_isolated(&mut g, orchard, (colors, false));
    g.state.zone_remove(Zone::graveyard(PlayerId(0)), red);
    g.state.update_obj(red, |o| o.zone = Zone::exile(PlayerId(0)));
    g.state.zone_push(Zone::exile(PlayerId(0)), red);
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green), false));
}

#[test]
fn a_stack_ability_with_a_tap_cost_is_not_tapping_for_mana() {
    let mut g = stack_game();
    let source = put(&mut g, &TARGETED, 1);
    add_object(&mut g, &GREEN_TARGET, PlayerId(1), Zone::graveyard(PlayerId(1)));
    put(&mut g, &CONTAMINATION, 0);
    let AbilityDef::Activated(def) = g.cards.ability(g.chars(source).abilities().next().unwrap()) else { unreachable!() };
    assert!(!def.is_mana_ability());
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green), false));
}

fn printed_game() -> Game {
    let mut defs = vec![&FILLER, &CONTAMINATION, &ORCHARD];
    for name in ["Deathrite Shaman", "Life and Limb", "Artificial Evolution", "Lotus Cobra", "Soulbright Seeker", "Culber and Stamets, Star-Crossed", "Molten-Core Maestro", "Time Warp", "Crumbling Vestige", "Omnath, Locus of Creation", "Raphael, Ninja Destroyer", "Quintorius Kand", "Tamiyo, Inquisitive Student", "Cheering Crowd", "Coalition Relic", "Firebending Student", "Wizard's Rockets", "Dal R'El, Augment Orphan", "Ashling, Rekindled", "\"Name Sticker\" Goblin"] {
        defs.push(cantrip_cards::all().into_iter().find(|d| d.name == name).expect("printed prerequisite definition"));
    }
    for name in ["Damping Sphere", "Vivi Ornitier", "Desolation of Smaug", "Ancient Tomb"] {
        defs.push(cantrip_cards::all().into_iter().find(|d| d.name == name).expect("printed recipe/replacement peer"));
    }
    let cards = Box::leak(Box::new(CardTable::new(defs)));
    Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&FILLER); 20]; 2], ..Default::default() })
}

fn printed(_: &Game, name: &str) -> &'static CardDef {
    cantrip_cards::all().into_iter().find(|d| d.name == name).unwrap()
}

fn deathrite_as_forest(g: &mut Game) -> ObjectId {
    let shaman = put(g, printed(g, "Deathrite Shaman"), 1);
    let limb = put(g, printed(g, "Life and Limb"), 0);
    assert!(!g.chars(shaman).is(CardType::Land), "unchanged Life and Limb does not animate Elves");
    let evolution = printed(g, "Artificial Evolution");
    let spell = add_object(g, evolution, PlayerId(0), Zone::hand(PlayerId(0)));
    let item = crate::actions::stack_item(StackKind::Spell, spell, AbilityRef::spell(g.cards.id_of(evolution)),
        PlayerId(0), &[Target::Object(limb)]);
    // Apply the actual printed spell, with the exact text choices, rather than approximating the layer result.
    in_frame(g, cantrip_state::FrameKind::Settle, vec![Answer::Subtype(Subtype::Saproling), Answer::Subtype(Subtype::Elf)], |cx| {
        cx.acting_as(PlayerId(0), |cx| (evolution.spell.unwrap().effect)(cx, &item)).unwrap();
    });
    assert!(g.chars(shaman).is(CardType::Land));
    assert!(g.chars(shaman).has_subtype(Subtype::Forest));
    shaman
}

#[test]
fn actual_deathrite_as_forest_requires_a_legal_land_target_and_keeps_its_stack_classification() {
    let mut g = printed_game();
    let shaman = deathrite_as_forest(&mut g);
    let def = g.chars(shaman).abilities().find_map(|a| match g.cards.ability(a) {
        AbilityDef::Activated(def) if def.produces_mana => Some(def), _ => None,
    }).expect("Deathrite's printed land-exiling ability survives becoming a Forest");
    assert!(!def.is_mana_ability(), "Deathrite's targeted ability uses the stack");
    assert_capability_isolated(&mut g, shaman, (ColorSet::of(Color::Green), false));
    add_object(&mut g, &FILLER, PlayerId(0), Zone::graveyard(PlayerId(0)));
    g.state.update_obj(shaman, |o| o.tapped = true);
    assert_capability_isolated(&mut g, shaman, (ColorSet(0b11111), false));
    put(&mut g, &CONTAMINATION, 0);
    assert_capability_isolated(&mut g, shaman, (ColorSet(0b11111), false));
    let orchard = put(&mut g, &ORCHARD, 0);
    assert_capability_isolated(&mut g, orchard, (ColorSet::of(Color::Black), false));
}

fn activate_deathrite(g: &mut Game, shaman: ObjectId, land: ObjectId) -> u32 {
    g.state.update_turn(|turn| { turn.number = 2; turn.active = PlayerId(1); turn.priority = PlayerId(1); turn.set_step(Step::PrecombatMain); });
    g.state.update_player(PlayerId(1), |player| player.turn_began = 2);
    let index = g.chars(shaman).abilities().position(|a| matches!(g.cards.ability(a), AbilityDef::Activated(d) if d.produces_mana)).unwrap();
    let mut action = Action::of(&ActionOption::on(ActionKind::ActivateAbility, shaman, index as u8));
    action.targets = vec![Target::Object(land)];
    in_frame(g, cantrip_state::FrameKind::Settle, vec![Answer::Act(action)], |cx| crate::actions::act(cx, PlayerId(1))).unwrap();
    g.state.stack.last().unwrap().serial
}

#[test]
fn actual_deathrite_activation_waits_on_stack_then_adds_the_chosen_mana_without_tap_replacement() {
    let mut g = printed_game();
    let shaman = deathrite_as_forest(&mut g);
    put(&mut g, &CONTAMINATION, 0);
    let land = add_object(&mut g, &FILLER, PlayerId(0), Zone::graveyard(PlayerId(0)));
    let serial = activate_deathrite(&mut g, shaman, land);
    assert!(g.state.obj(shaman).tapped);
    assert!(g.state.player(PlayerId(1)).pool.is_empty(), "activation produces no mana before resolution");
    assert_eq!(g.state.stack.len(), 1);
    assert_eq!(g.state.obj(land).zone, Zone::graveyard(PlayerId(0)));
    in_frame(&mut g, cantrip_state::FrameKind::Resolve { serial }, vec![Answer::Colors(vec![Color::Red])], crate::resolve::resolve).unwrap();
    assert!(g.state.stack.is_empty());
    assert_eq!(g.state.player(PlayerId(1)).pool.0, [0, 0, 0, 1, 0, 0], "Contamination does not replace Deathrite's stack mana");
    let exiled = g.state.obj(land).became.expect("exile creates a new object (CR 400.7)");
    assert!(g.state.obj(exiled).zone.is(ZoneKind::Exile));
    assert!(!g.log.iter().any(|record| matches!(record.event, Event::TappedForMana { permanent, .. } if permanent == shaman)));
}

#[test]
fn actual_deathrite_with_a_departed_target_resolves_without_choosing_or_producing_mana() {
    let mut g = printed_game();
    let shaman = deathrite_as_forest(&mut g);
    let land = add_object(&mut g, &FILLER, PlayerId(0), Zone::graveyard(PlayerId(0)));
    let serial = activate_deathrite(&mut g, shaman, land);
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx| {
        cx.acting_as(PlayerId(0), |cx| cantrip_api::Game::exile(cx, land));
    });
    in_frame(&mut g, cantrip_state::FrameKind::Resolve { serial }, vec![], crate::resolve::resolve).unwrap();
    assert!(g.state.stack.is_empty());
    assert!(g.state.player(PlayerId(1)).pool.is_empty());
    assert!(g.state.agenda.last().unwrap().pending.is_none(), "no color choice after the target is gone");
}

fn dependent_stack_mana(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let colors = own_colors(g, &Asker::new(item.object, item.controller));
    if !colors.is_empty() {
        let Answer::Colors(chosen) = g.ask(item.controller, RequestKind::ChooseColors {
            source: item.object, choices: colors, count: 1, purpose: ColorUse::AddMana,
        })? else { unreachable!() };
        g.add_mana(item.controller, chosen[0].mana(), 1);
    }
    Ok(())
}

static STACK_DEPENDENT: CardDef = CardDef {
    name: "Capability mana-producing loyalty land",
    abilities: &[ability(loyalty(1), dependent_stack_mana, "+1: Add one mana of any color a land you control could produce.").produces_mana()],
    ..FILLER
};

#[test]
fn recursive_stack_production_needs_an_independent_seed_and_retains_replacements_on_that_seed() {
    let cards = Box::leak(Box::new(CardTable::new([&FILLER, &STACK_DEPENDENT, &GREEN, &CONTAMINATION])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&FILLER); 20]; 2], ..Default::default() });
    let source = put(&mut g, &STACK_DEPENDENT, 1);
    assert_capability_isolated(&mut g, source, (ColorSet::NONE, false));
    put(&mut g, &GREEN, 1);
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green), false));
    put(&mut g, &CONTAMINATION, 0);
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Black), false));
}

fn sequential_stack_mana(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::G, 1);
    if g.state().player(item.controller).pool.0[Mana::G as usize] > 0 {
        g.add_mana(item.controller, Mana::R, 1);
    }
    Ok(())
}

#[test]
fn hypothetical_stack_instructions_see_mana_added_earlier_in_the_same_resolution() {
    static SEQUENTIAL: CardDef = CardDef {
        name: "Capability sequential loyalty land",
        abilities: &[ability(loyalty(1), sequential_stack_mana, "+1: Add {G}, then if you have green mana, add {R}.").produces_mana()],
        ..FILLER
    };
    let cards = Box::leak(Box::new(CardTable::new([&FILLER, &SEQUENTIAL])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&FILLER); 20]; 2], ..Default::default() });
    let source = put(&mut g, &SEQUENTIAL, 1);
    assert_capability_isolated(&mut g, source, (ColorSet(ColorSet::of(Color::Green).0 | ColorSet::of(Color::Red).0), false));
}

fn opponents_colors(q: &dyn Query, me: &Asker) -> ColorSet {
    let mut colors = ColorSet::NONE;
    for opponent in q.state().opponents(me.controller) {
        for land in q.permanents_of(opponent) {
            if q.chars(land).is(CardType::Land) {
                colors.0 |= q.producible_types(land).0.0;
            }
        }
    }
    colors
}

fn own_colors(q: &dyn Query, me: &Asker) -> ColorSet {
    ColorSet(q.permanents_of(me.controller).into_iter().filter(|&id| q.chars(id).is(CardType::Land))
        .fold(0, |colors, id| colors | q.producible_types(id).0.0))
}

// Reflecting Pool's colorless branch is needed when querying its production even though these tests never activate it.
fn pool_mana(g: &mut dyn cantrip_api::Game, me: &Asker, color: Option<Color>) -> Result<(), Suspend> {
    if let Some(color) = color {
        g.add_mana(me.controller, color.mana(), 1);
    } else if g.permanents_of(me.controller).into_iter()
        .any(|id| g.chars(id).is(CardType::Land) && g.producible_types(id).1)
    {
        g.add_mana(me.controller, Mana::C, 1);
    }
    Ok(())
}

fn conditional_mana(g: &mut dyn cantrip_api::Game, me: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    if g.obj(me.source).counters(CounterKind::Charge) > 0 {
        g.add_mana(me.controller, Mana::G, 1);
    }
    Ok(())
}

fn land_tapped(q: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ManaProduction(m) if q.chars(m.permanent).is(CardType::Land))
}

fn land_source(q: &dyn Query, _: &Asker, permanent: ObjectId, _: PlayerId) -> bool {
    q.chars(permanent).is(CardType::Land)
}

fn black_instead(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ManaProduction(m) = proposal {
        m.mana = ManaPool::default();
        m.mana.0[Mana::B as usize] = 1;
    }
}

static FILLER: CardDef = CardDef { name: "Capability filler", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static GREEN: CardDef = CardDef {
    name: "Capability Forest", abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")], ..FILLER
};
static COLORLESS: CardDef = CardDef {
    name: "Capability Wastes", abilities: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")], ..FILLER
};
static ORCHARD: CardDef = CardDef {
    name: "Capability Exotic Orchard",
    abilities: &[mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color that a land an opponent controls could produce.")
        .colors_from(opponents_colors)], ..FILLER
};
static POOL: CardDef = CardDef {
    name: "Capability Reflecting Pool",
    abilities: &[mana_ability(TAP, add_by(pool_mana).choose_color(), "{T}: Add one mana of any type that a land you control could produce.")
        .colors_from(own_colors)], ..FILLER
};
static COSTLY: CardDef = CardDef {
    name: "Capability costly land",
    abilities: &[mana_ability(cost("{99}, {T}").and(remove_counters(CounterKind::Charge, 1)),
        add_any_color(1).only_for(ManaRestriction::CREATURE_SPELLS), "Add one mana of any color.")
        .condition(|_, _| false)], ..FILLER
};
static CONDITIONAL: CardDef = CardDef {
    name: "Capability conditional land",
    abilities: &[mana_ability(TAP, add_by(conditional_mana), "If this land has a charge counter, add {G}.")], ..FILLER
};
static CONTAMINATION: CardDef = CardDef {
    name: "Capability Contamination", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[fixed_mana_replacement(1, land_source, Mana::B, 1, "produces {B} instead")],
    ..CardDef::BLANK
};

fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([&FILLER, &GREEN, &COLORLESS, &ORCHARD, &POOL, &COSTLY, &CONDITIONAL, &CONTAMINATION])));
    Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&FILLER); 20]; 3], ..Default::default() })
}

fn put(g: &mut Game, def: &'static CardDef, player: u8) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), PlayerId(player), Zone::BATTLEFIELD));
    let timestamp = g.state.timestamp();
    g.state.update_obj(id, |o| o.timestamp = timestamp);
    g.state.zone_push(Zone::BATTLEFIELD, id);
    g.state.see_statics();
    id
}

#[test]
fn capability_ignores_costs_activation_restrictions_and_mana_spending_restrictions() {
    let mut g = game();
    let costly = put(&mut g, &COSTLY, 1);
    g.state.update_obj(costly, |o| o.tapped = true);
    let log_len = g.log.len();
    assert_eq!(g.producible_types(costly), (ColorSet(0b11111), false));
    assert_eq!(g.log.len(), log_len);
    assert!(g.state.obj(costly).tapped);
    assert_eq!(g.state.obj(costly).counters(CounterKind::Charge), 0);
    assert!(g.state.player(PlayerId(1)).pool.is_empty());
}

#[test]
fn conditional_effects_read_current_state_and_query_side_effects_are_isolated() {
    let mut g = game();
    let land = put(&mut g, &CONDITIONAL, 1);
    assert_eq!(g.producible_types(land), (ColorSet::NONE, false));
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx| {
        cx.acting_as(PlayerId(1), |cx| cantrip_api::Game::add_counters(cx, land, CounterKind::Charge, 1));
    });
    assert_eq!(g.producible_types(land), (ColorSet::of(Color::Green), false));
    assert_eq!(g.state.obj(land).counters(CounterKind::Charge), 1);
    assert!(!g.state.obj(land).tapped);
    assert!(g.state.player(PlayerId(1)).pool.is_empty());
    assert!(g.producible_path.borrow().is_empty());
}

#[test]
fn recursive_orchards_and_reflecting_pool_need_an_independent_seed_and_do_not_cache_stale_colors() {
    let mut g = game();
    let ours = put(&mut g, &ORCHARD, 0);
    let theirs = put(&mut g, &ORCHARD, 1);
    let pool = put(&mut g, &POOL, 1);
    for id in [ours, theirs, pool] {
        assert_eq!(g.producible_types(id), (ColorSet::NONE, false), "unseeded dependency");
    }
    let forest = put(&mut g, &GREEN, 0);
    for id in [ours, theirs, pool] {
        assert_eq!(g.producible_types(id), (ColorSet::of(Color::Green), false), "seeded dependency");
    }
    g.state.zone_remove(Zone::BATTLEFIELD, forest);
    for id in [ours, theirs, pool] {
        assert_eq!(g.producible_types(id), (ColorSet::NONE, false), "removed seed");
    }
    assert!(g.producible_path.borrow().is_empty());
}

#[test]
fn recursive_dependencies_apply_replacements_before_propagating_types() {
    let mut g = game();
    let ours = put(&mut g, &ORCHARD, 0);
    put(&mut g, &ORCHARD, 1);
    put(&mut g, &CONTAMINATION, 2);
    assert_eq!(g.producible_types(ours), (ColorSet::NONE, false), "replacements cannot invent a production");
    put(&mut g, &GREEN, 0);
    assert_eq!(g.producible_types(ours), (ColorSet::of(Color::Black), false));
}

#[test]
fn raw_state_reader_and_frame_query_agree_with_the_games_query() {
    let mut g = game();
    let land = put(&mut g, &COSTLY, 1);
    let reader = crate::resolve::Reader { cards: g.cards, state: &g.state, events: &g.log };
    assert_eq!(reader.producible_types(land), g.producible_types(land));
    let cx = crate::Cx { g: &mut g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None };
    assert_eq!(cx.producible_types(land), (ColorSet(0b11111), false));
}

#[test]
fn reflective_dependencies_preserve_colorless_but_opponent_color_dependencies_do_not() {
    let mut g = game();
    let pool = put(&mut g, &POOL, 1);
    put(&mut g, &COLORLESS, 1);
    let orchard = put(&mut g, &ORCHARD, 0);
    assert_eq!(g.producible_types(pool), (ColorSet::NONE, true));
    assert_eq!(g.producible_types(orchard), (ColorSet::NONE, false));
}


// CR 106.7 has no target-count limit; enumerate all 43 real legal land targets.
#[test]
fn actual_deathrite_as_forest_with_43_legal_land_targets_has_all_five_capability_colors() {
    let mut g = printed_game();
    let shaman = deathrite_as_forest(&mut g);
    for _ in 0..43 {
        add_object(&mut g, &FILLER, PlayerId(0), Zone::graveyard(PlayerId(0)));
    }
    assert_capability_isolated(&mut g, shaman, (ColorSet(0b11111), false));
}

// CR 106.7 includes the actual Cobra's Landfall production after the actual text/layer change.
#[test]
fn actual_lotus_cobra_as_forest_includes_its_triggered_mana_production() {
    let mut g = printed_game();
    let cobra_def = printed(&g, "Lotus Cobra");
    let cobra = put(&mut g, cobra_def, 1);
    let limb_def = printed(&g, "Life and Limb");
    let limb = put(&mut g, limb_def, 0);
    assert!(!g.chars(cobra).is(CardType::Land));
    let evolution = printed(&g, "Artificial Evolution");
    let spell = add_object(&mut g, evolution, PlayerId(0), Zone::hand(PlayerId(0)));
    let item = crate::actions::stack_item(StackKind::Spell, spell, AbilityRef::spell(g.cards.id_of(evolution)),
        PlayerId(0), &[Target::Object(limb)]);
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![Answer::Subtype(Subtype::Saproling), Answer::Subtype(Subtype::Snake)], |cx| {
        cx.acting_as(PlayerId(0), |cx| (evolution.spell.unwrap().effect)(cx, &item)).unwrap();
    });
    assert!(g.chars(cobra).is(CardType::Land));
    assert!(g.chars(cobra).has_subtype(Subtype::Forest));
    assert_capability_isolated(&mut g, cobra, (ColorSet(0b11111), false));
}


/// CR 603.3/605.1b: a real land entry triggers Cobra, which adds nothing until its stack ability resolves.
#[test]
fn actual_lotus_cobra_landfall_waits_on_stack_and_is_not_tapping_for_mana() {
    let mut g = printed_game();
    let cobra_def = printed(&g, "Lotus Cobra");
    let cobra = put(&mut g, cobra_def, 1);
    put(&mut g, &CONTAMINATION, 0);
    let AbilityDef::Triggered(trigger) = g.cards.ability(g.chars(cobra).abilities().next().unwrap()) else { unreachable!() };
    assert!(trigger.produces_mana);
    assert!(!trigger.mana_ability, "Landfall is an ordinary stack trigger (CR 605.1b)");
    assert_capability_isolated(&mut g, cobra, (ColorSet(0b11111), false));
    assert!(g.state.triggers.is_empty(), "a capability query does not fabricate a land entry");
    assert!(g.state.stack.is_empty());
    let land = add_object(&mut g, &FILLER, PlayerId(1), Zone::hand(PlayerId(1)));
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx| {
        cantrip_api::Game::put_onto_battlefield(cx, land, PlayerId(1), false).unwrap();
    });
    assert_eq!(g.state.triggers.len(), 1, "the real entry produces exactly one trigger");
    assert_eq!(g.state.triggers[0].source, cobra);
    assert!(g.state.player(PlayerId(1)).pool.is_empty(), "triggering adds no mana");
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], crate::settle::settle).unwrap();
    assert_eq!(g.state.stack.len(), 1);
    let item = *g.state.stack.last().unwrap();
    assert_eq!(item.kind, StackKind::Triggered);
    assert_eq!(item.object, cobra);
    assert!(g.state.player(PlayerId(1)).pool.is_empty(), "putting it on the stack adds no mana");
    assert_capability_isolated(&mut g, cobra, (ColorSet(0b11111), false));
    in_frame(&mut g, cantrip_state::FrameKind::Resolve { serial: item.serial },
        vec![Answer::Colors(vec![Color::Red])], crate::resolve::resolve).unwrap();
    assert!(g.state.stack.is_empty());
    assert_eq!(g.state.player(PlayerId(1)).pool, ManaPool([0, 0, 0, 1, 0, 0]));
    assert!(!g.state.obj(cobra).tapped);
    assert!(!g.log.iter().any(|record| matches!(record.event, Event::TappedForMana { permanent, .. } if permanent == cobra)),
        "ordinary triggered production is not a tap for mana");
}

/// CR 603.4/106.7: intervening conditions and legal targets limit production, trigger-only restrictions do not.
#[test]
fn triggered_capability_checks_intervening_conditions_legal_targets_and_effect_choices() {
    static TRIGGER_TARGET: CardDef = CardDef {
        name: "Capability conditional targeted trigger land",
        abilities: &[triggered(cantrip_mechanics::triggers::LANDFALL, targeted_mana,
            "Whenever a land enters, if this has a charge counter, exile target land card and add one mana of one of its colors.")
            .targets(&[target(grave_land)]).produces_mana()
            .condition(|q, me| q.state().obj(me.source).counters(CounterKind::Charge) > 0)
            .only_if(|_, _| false)],
        ..FILLER
    };
    let cards = Box::leak(Box::new(CardTable::new([&FILLER, &TRIGGER_TARGET, &GREEN_TARGET, &RED_TARGET, &CONTAMINATION])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&FILLER); 20]; 2], ..Default::default() });
    let source = put(&mut g, &TRIGGER_TARGET, 1);
    let green = add_object(&mut g, &GREEN_TARGET, PlayerId(0), Zone::graveyard(PlayerId(0)));
    add_object(&mut g, &RED_TARGET, PlayerId(1), Zone::graveyard(PlayerId(1)));
    assert_capability_isolated(&mut g, source, (ColorSet::NONE, false));
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx| {
        cx.acting_as(PlayerId(1), |cx| cantrip_api::Game::add_counters(cx, source, CounterKind::Charge, 1));
    });
    put(&mut g, &CONTAMINATION, 0);
    assert_capability_isolated(&mut g, source, (ColorSet(ColorSet::of(Color::Green).0 | ColorSet::of(Color::Red).0), false));
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx| { cx.acting_as(PlayerId(0), |cx| cantrip_api::Game::exile(cx, green)); });
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Red), false));
}

fn linked_trigger_mana(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(object) = g.state().chosen_object(g.linked(item)) {
        for color in Color::ALL {
            if g.chars(object).colors.has(color) {
                g.add_mana(item.controller, color.mana(), 1);
            }
        }
    }
    Ok(())
}

/// CR 607.5: a granted ability cannot use the receiver's own choice; its own acquired link can supply a different color.
#[test]
fn triggered_capability_preserves_each_acquired_link_for_conditions_and_resolution() {
    static LINKED_TRIGGER: CardDef = CardDef {
        name: "Capability linked trigger land",
        abilities: &[triggered(cantrip_mechanics::triggers::LANDFALL, linked_trigger_mana, "Whenever a land enters, add mana of the chosen object's colors.")
            .produces_mana().condition(|q, me| q.state().chosen_object(q.linked_to(me)).is_some())],
        ..FILLER
    };
    let cards = Box::leak(Box::new(CardTable::new([&FILLER, &LINKED_TRIGGER, &GREEN_TARGET, &RED_TARGET])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&FILLER); 20]; 2], ..Default::default() });
    let source = put(&mut g, &FILLER, 1);
    let lender = put(&mut g, &LINKED_TRIGGER, 0);
    let green = add_object(&mut g, &GREEN_TARGET, PlayerId(0), Zone::hand(PlayerId(0)));
    let red = add_object(&mut g, &RED_TARGET, PlayerId(1), Zone::hand(PlayerId(1)));
    let choices = <cantrip_state::Choices as cantrip_state::ChoiceRecord>::entered(cards.id_of(&FILLER),
        cantrip_state::Entered { object: Some(green), ..Default::default() });
    g.state.update_obj(source, |o| o.marks.choices = choices);
    let ability = AbilityRef { card: cards.id_of(&LINKED_TRIGGER), index: 0 };
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx| {
        cantrip_api::Game::add_effect(cx, lender, source, EffectKind::AddAbility(ability), Duration::EndOfTurn);
    });
    assert!(g.chars(source).abilities().any(|a| a == ability), "the receiver really has the ability");
    assert_capability_isolated(&mut g, source, (ColorSet::NONE, false));
    let chars = g.chars(source);
    let slot = chars.abilities.iter().position(|a| *a == Some(ability)).unwrap();
    let acquired = crate::layers::acquired(cards, &g.state, &g.log, source, &chars, slot);
    assert_ne!(acquired, Acquired::OWN);
    let by = Linked { object: source, text: cards.linked_card(ability.card), acquired };
    // The other part of this acquired pair made its own object choice.
    g.state.update_obj(source, |o| {
        assert!(cantrip_state::ChoiceRecord::keep(&mut o.marks.choices, by, cantrip_state::Kept::Object(red)));
    });
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Red), false));
}

fn ordered_colors_mana(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let Answer::Colors(colors) = g.ask(item.controller, RequestKind::ChooseColors {
        source: item.object, choices: ColorSet(ColorSet::of(Color::Red).0 | ColorSet::of(Color::Green).0),
        count: 2, purpose: ColorUse::AddMana,
    })? else { unreachable!() };
    if colors.contains(&Color::Green) && colors.contains(&Color::Red) {
        g.add_mana(item.controller, Mana::W, 1);
    }
    Ok(())
}

/// Colors chosen together are a multiset (CR 106.1b): green and red is tried once, in whatever order, and found.
#[test]
fn hypothetical_color_choices_are_tried_as_multisets() {
    static ORDERED: CardDef = CardDef {
        name: "Capability color choices land",
        abilities: &[ability(loyalty(1), ordered_colors_mana, "+1: Choose two colors; add white only for green and red.").produces_mana()],
        ..FILLER
    };
    let cards = Box::leak(Box::new(CardTable::new([&FILLER, &ORDERED])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&FILLER); 20]; 2], ..Default::default() });
    let source = put(&mut g, &ORDERED, 1);
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::White), false));
}


/// A rare color witness is processed after 300 other targets; neither target menus nor total trials may truncate it.
#[test]
fn legal_target_menus_above_256_keep_a_late_distinct_color_witness() {
    let mut g = stack_game();
    let source = put(&mut g, &TARGETED, 1);
    add_object(&mut g, &RED_TARGET, PlayerId(0), Zone::graveyard(PlayerId(0)));
    for _ in 0..300 {
        add_object(&mut g, &GREEN_TARGET, PlayerId(0), Zone::graveyard(PlayerId(0)));
    }
    assert_capability_isolated(&mut g, source, (ColorSet(ColorSet::of(Color::Green).0 | ColorSet::of(Color::Red).0), false));
}

// Printed producers with modes, history and triggering-spell context are regression witnesses for CR 106.7.
fn printed_creature_as_forest(g: &mut Game, name: &str, subtype: Subtype) -> ObjectId {
    let source = put(g, printed(g, name), 1);
    let limb = put(g, printed(g, "Life and Limb"), 0);
    let evolution = printed(g, "Artificial Evolution");
    let spell = add_object(g, evolution, PlayerId(0), Zone::hand(PlayerId(0)));
    let item = crate::actions::stack_item(StackKind::Spell, spell, AbilityRef::spell(g.cards.id_of(evolution)),
        PlayerId(0), &[Target::Object(limb)]);
    in_frame(g, cantrip_state::FrameKind::Settle, vec![Answer::Subtype(Subtype::Saproling), Answer::Subtype(subtype)], |cx| {
        cx.acting_as(PlayerId(0), |cx| (evolution.spell.unwrap().effect)(cx, &item)).unwrap();
    });
    assert!(g.chars(source).is(CardType::Land));
    assert!(g.chars(source).has_subtype(Subtype::Forest));
    source
}

#[test]
fn actual_culber_and_stamets_as_forest_explores_its_modal_trigger_without_creating_an_entry() {
    let mut g = printed_game();
    let source = printed_creature_as_forest(&mut g, "Culber and Stamets, Star-Crossed", Subtype::Human);
    let def = g.chars(source).abilities().find_map(|a| match g.cards.ability(a) {
        AbilityDef::Triggered(d) if d.produces_mana => Some(d), _ => None,
    }).expect("the annotated modal trigger survives becoming a Forest");
    assert!(def.modal.is_some() && !def.mana_ability);
    assert!(g.state.triggers.is_empty());
    assert_capability_isolated(&mut g, source, (ColorSet(0b11111), false));
    put(&mut g, &CONTAMINATION, 0);
    assert_capability_isolated(&mut g, source, (ColorSet(0b11111), false));
}

fn prepare_main(g: &mut Game) {
    g.state.update_turn(|turn| { turn.number = 2; turn.active = PlayerId(1); turn.priority = PlayerId(1); turn.set_step(Step::PrecombatMain); });
    g.state.update_player(PlayerId(1), |p| p.turn_began = 2);
}

#[test]
fn actual_soulbright_as_forest_reads_real_resolution_history_without_counting_a_query_as_a_resolution() {
    let mut g = printed_game();
    let source = printed_creature_as_forest(&mut g, "Soulbright Seeker", Subtype::Elemental);
    prepare_main(&mut g);
    let ability = g.chars(source).abilities().find(|&a| matches!(g.cards.ability(a), AbilityDef::Activated(d) if d.produces_mana)).unwrap();
    let index = g.chars(source).abilities().position(|a| a == ability).unwrap();
    assert!(!matches!(g.cards.ability(ability), AbilityDef::Activated(d) if d.is_mana_ability()));
    for previous in 0..3 {
        let expected = if previous == 2 { ColorSet::of(Color::Green).with(Color::Red) } else { ColorSet::of(Color::Green) };
        assert_capability_isolated(&mut g, source, (expected, false));
        in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx| {
            cx.acting_as(PlayerId(1), |cx| cantrip_api::Game::add_mana(cx, PlayerId(1), Mana::R, 1));
        });
        let mut action = Action::of(&ActionOption::on(ActionKind::ActivateAbility, source, index as u8));
        action.targets = vec![Target::Object(source)];
        let result = in_frame(&mut g, cantrip_state::FrameKind::Settle,
            vec![Answer::Act(action), Answer::Act(Action::of(&ActionOption::simple(ActionKind::ConfirmCasting)))],
            |cx| crate::actions::act(cx, PlayerId(1)));
        result.unwrap();
        assert!(g.state.player(PlayerId(1)).pool.is_empty(), "only the activation cost has been paid");
        let serial = g.state.stack.last().unwrap().serial;
        in_frame(&mut g, cantrip_state::FrameKind::Resolve { serial }, vec![], crate::resolve::resolve).unwrap();
        assert_eq!(g.state.player(PlayerId(1)).pool.0[Mana::R as usize], if previous == 2 { 4 } else { 0 });
    }
    assert_eq!(g.history().resolutions(source, ability), 3);
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green), false));
}

#[test]
fn actual_molten_core_as_forest_keeps_the_real_cast_spell_context_while_pending_and_on_stack() {
    let mut g = printed_game();
    let source = printed_creature_as_forest(&mut g, "Molten-Core Maestro", Subtype::Goblin);
    prepare_main(&mut g);
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green), false));
    let warp = printed(&g, "Time Warp");
    let card = add_object(&mut g, warp, PlayerId(1), Zone::hand(PlayerId(1)));
    // Supply the payment position, then use the actual casting procedure and its real triggering event.
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx| {
        cx.acting_as(PlayerId(1), |cx| {
            cantrip_api::Game::add_mana(cx, PlayerId(1), Mana::U, 2);
            cantrip_api::Game::add_mana(cx, PlayerId(1), Mana::C, 3);
        });
    });
    let mut action = Action::of(&ActionOption::on(ActionKind::CastSpell, card, 0));
    action.targets = vec![Target::Player(PlayerId(1))];
    in_frame(&mut g, cantrip_state::FrameKind::Settle,
        vec![Answer::Act(action), Answer::Act(Action::of(&ActionOption::simple(ActionKind::ConfirmCasting)))],
        |cx| crate::actions::act(cx, PlayerId(1))).unwrap();
    assert!(g.state.player(PlayerId(1)).pool.is_empty());
    assert_eq!(g.state.triggers.len(), 1, "the real five-mana instant/sorcery cast triggers Opus");
    let context = g.state.triggers[0];
    assert_eq!(context.source, source);
    let Target::Object(spell) = context.subject else { panic!("Opus carries the real spell") };
    assert_eq!(g.state.obj(spell).spent.total(), 5);
    let expected = (ColorSet::of(Color::Green).with(Color::Red), false);
    assert_capability_isolated(&mut g, source, expected);
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], crate::settle::settle).unwrap();
    assert!(g.state.triggers.is_empty());
    assert_eq!(g.state.stack.last().unwrap().subject, Target::Object(spell));
    assert_capability_isolated(&mut g, source, expected);
    let serial = g.state.stack.last().unwrap().serial;
    in_frame(&mut g, cantrip_state::FrameKind::Resolve { serial }, vec![], crate::resolve::resolve).unwrap();
    assert!(g.state.player(PlayerId(1)).pool.0[Mana::R as usize] > 0);
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green), false));
}

fn choice_game(defs: impl IntoIterator<Item = &'static CardDef>) -> Game {
    let cards = Box::leak(Box::new(CardTable::new(defs)));
    Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&FILLER); 20]; 2], ..Default::default() })
}

fn add_one_green(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> { g.add_mana(item.controller, Mana::G, 1); Ok(()) }
fn add_one_red(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> { g.add_mana(item.controller, Mana::R, 1); Ok(()) }

#[test]
fn modal_activated_capability_ignores_mode_and_escalate_costs_and_explores_repeated_modes() {
    static MODAL: ModalDef = ModalDef {
        min: 2, max: 2, repeat: true, escalate: Some(cost("{99}").and(DISCARD_THIS)),
        ..modes(&[mode(add_one_green, "Add green."), mode(add_one_red, "Add red.")])
    };
    static SOURCE: CardDef = CardDef { name: "Capability repeated modal activation",
        abilities: &[modal_ability(cost("{99}"), &MODAL, "Choose two; a mode may be chosen twice.").produces_mana()], ..FILLER };
    let mut g = choice_game([&FILLER, &SOURCE]);
    let source = put(&mut g, &SOURCE, 1);
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green).with(Color::Red), false));
    // Distinguish repetition from selecting only both modes: two greens open a separate white output.
    static WITH_SHARED: ModalDef = ModalDef { shared: Some(|g, item| {
        if g.state().player(item.controller).pool.0[Mana::G as usize] == 2 { g.add_mana(item.controller, Mana::W, 1); }
        Ok(())
    }), ..MODAL };
    static REPEATED: CardDef = CardDef { name: "Capability repeated-mode witness",
        abilities: &[modal_ability(cost("{99}"), &WITH_SHARED, "Choose two; a mode may be chosen twice.").produces_mana()], ..FILLER };
    let mut g = choice_game([&FILLER, &REPEATED]);
    let source = put(&mut g, &REPEATED, 1);
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green).with(Color::Red).with(Color::White), false));
}

fn divided_witness(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let targets = g.state().targets(item);
    let shares = g.state().shares(item);
    if targets.len() == 2 && shares == [1, 2]
        && targets[0].object().is_some_and(|id| g.chars(id).colors.has(Color::Green))
        && targets[1].object().is_some_and(|id| g.chars(id).colors.has(Color::Red)) {
        g.add_mana(item.controller, Mana::W, 1);
    }
    Ok(())
}

#[test]
fn divided_activated_and_triggered_capabilities_keep_ordered_targets_and_every_positive_share() {
    static SPECS: &[TargetSpec] = &[TargetSpec { min: 1, max: 2, ..target(grave_land) }];
    static ACTIVATED: CardDef = CardDef { name: "Capability divided activation",
        abilities: &[ability(cost("{99}"), divided_witness, "Divide three among up to two targets; conditionally add white.")
            .targets(SPECS).divide(Divide { total: 3, reads: None }).produces_mana()], ..FILLER };
    static TRIGGER: CardDef = CardDef { name: "Capability divided trigger",
        abilities: &[triggered(cantrip_mechanics::triggers::LANDFALL, divided_witness, "Divide three among up to two targets; conditionally add white.")
            .targets(SPECS).divide(Divide { total: 3, reads: None }).produces_mana()], ..FILLER };
    for def in [&ACTIVATED, &TRIGGER] {
        let mut g = choice_game([&FILLER, def, &GREEN_TARGET, &RED_TARGET]);
        let source = put(&mut g, def, 1);
        assert_capability_isolated(&mut g, source, (ColorSet::NONE, false));
        add_object(&mut g, &GREEN_TARGET, PlayerId(0), Zone::graveyard(PlayerId(0)));
        let red = add_object(&mut g, &RED_TARGET, PlayerId(1), Zone::graveyard(PlayerId(1)));
        assert_capability_isolated(&mut g, source, (ColorSet::of(Color::White), false));
        in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx| { cx.acting_as(PlayerId(1), |cx| cantrip_api::Game::exile(cx, red)); });
        assert_capability_isolated(&mut g, source, (ColorSet::NONE, false));
    }
}

fn exact_two(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    q.announced_x() == Some(2) && grave_land(q, asker, t)
}
fn bounded_x_mana(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if item.x == 2 { g.add_mana(item.controller, Mana::W, 1); }
    Ok(())
}

#[test]
fn required_counted_targets_bound_hypothetical_x_independently_of_mana_or_variable_tap_costs() {
    static SPECS: &[TargetSpec] = &[TargetSpec { min: TargetSpec::COUNTED, max: TargetSpec::COUNTED, ..target(exact_two) }];
    static MANA_X: CardDef = CardDef { name: "Capability bounded late X mana cost",
        abilities: &[ability(cost("{X}"), bounded_x_mana, "Choose X targets; add white if X is two.")
            .targets(SPECS).targets_after_x().produces_mana()], ..FILLER };
    static TAP_X: CardDef = CardDef { name: "Capability bounded late X tap cost",
        abilities: &[ability(Cost { variable: Some(VariableCost::TapX(|_, _, _| false)), ..Cost::NONE }, bounded_x_mana, "Tap X; choose X targets; add white if X is two.")
            .targets(SPECS).targets_after_x().produces_mana()], ..FILLER };
    for def in [&MANA_X, &TAP_X] {
        let mut g = choice_game([&FILLER, def, &GREEN_TARGET, &RED_TARGET]);
        let source = put(&mut g, def, 1);
        assert_capability_isolated(&mut g, source, (ColorSet::NONE, false));
        add_object(&mut g, &GREEN_TARGET, PlayerId(0), Zone::graveyard(PlayerId(0)));
        add_object(&mut g, &RED_TARGET, PlayerId(1), Zone::graveyard(PlayerId(1)));
        assert_capability_isolated(&mut g, source, (ColorSet::of(Color::White), false));
    }
}

#[test]
fn unbounded_x_reports_a_gap_and_restores_the_dependency_path_instead_of_assuming_zero() {
    static SOURCE: CardDef = CardDef { name: "Capability unbounded X regression",
        abilities: &[ability(cost("{X}"), bounded_x_mana, "Add white only if X is two.").produces_mana()], ..FILLER };
    let mut g = choice_game([&FILLER, &SOURCE]);
    let source = put(&mut g, &SOURCE, 1);
    let history = format!("{:?}", g.log);
    let at = g.at;
    // A gap is found under verification, so the first card to reach one fails its scenarios; in a game, what the ability
    // lists stands (KI-124) and nothing panics.
    let judged = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| g.producible_types(source)));
    if crate::layers::verifying() {
        let error = judged.expect_err("unbounded X must not be silently evaluated as zero");
        let text = error.downcast_ref::<String>().map(String::as_str).or_else(|| error.downcast_ref::<&str>().copied()).unwrap();
        assert!(text.contains("unbounded X needs a symbolic hypothetical effect judgment"), "{text}");
    } else {
        assert_eq!(judged.expect("no panic in a game"), (ColorSet::NONE, false));
    }
    assert_eq!(format!("{:?}", g.log), history);
    assert_eq!(g.at, at);
    assert!(g.producible_path.borrow().is_empty());
}


#[test]
fn actual_crumbling_vestige_has_five_entry_colors_and_colorless_even_tapped_without_an_entry_event() {
    let mut g = printed_game();
    let def = printed(&g, "Crumbling Vestige");
    let source = put(&mut g, def, 1);
    g.state.update_obj(source, |o| o.tapped = true);
    assert!(g.state.triggers.is_empty());
    assert_capability_isolated(&mut g, source, (ColorSet(0b11111), true));
    let orchard = put(&mut g, &ORCHARD, 0);
    assert_capability_isolated(&mut g, orchard, (ColorSet(0b11111), false));
    put(&mut g, &CONTAMINATION, 0);
    // The tap's C is replaced by B, while the ordinary entry trigger retains every color.
    assert_capability_isolated(&mut g, source, (ColorSet(0b11111), false));
}

#[test]
fn actual_omnath_as_forest_reads_real_landfall_resolution_history() {
    let mut g = printed_game();
    let source = printed_creature_as_forest(&mut g, "Omnath, Locus of Creation", Subtype::Elemental);
    prepare_main(&mut g);
    let ability = g.chars(source).abilities().find(|&a| matches!(g.cards.ability(a), AbilityDef::Triggered(d) if d.produces_mana)).unwrap();
    let only_green = (ColorSet::of(Color::Green), false);
    let second = (ColorSet::of(Color::Green).with(Color::Red).with(Color::White).with(Color::Blue), false);
    for previous in 0..3 {
        assert_capability_isolated(&mut g, source, if previous == 1 { second } else { only_green });
        let land = add_object(&mut g, &FILLER, PlayerId(1), Zone::hand(PlayerId(1)));
        in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx| {
            cx.acting_as(PlayerId(1), |cx| cantrip_api::Game::put_onto_battlefield(cx, land, PlayerId(1), false)).unwrap();
        });
        assert_eq!(g.state.triggers.len(), 1);
        in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], crate::settle::settle).unwrap();
        let item = *g.state.stack.last().unwrap();
        assert_eq!(item.object, source);
        let before_life = g.state.player(PlayerId(1)).life;
        in_frame(&mut g, cantrip_state::FrameKind::Resolve { serial: item.serial }, vec![], crate::resolve::resolve).unwrap();
        assert_eq!(g.history().resolutions(source, ability), previous + 1);
        assert_eq!(g.state.player(PlayerId(1)).life, before_life + if previous == 0 { 4 } else { 0 });
    }
    assert_capability_isolated(&mut g, source, only_green);
}

#[test]
fn actual_raphael_as_forest_keeps_real_damage_amount_while_pending_and_on_stack() {
    let mut g = printed_game();
    let source = printed_creature_as_forest(&mut g, "Raphael, Ninja Destroyer", Subtype::Turtle);
    prepare_main(&mut g);
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green), false));
    let dealer = put(&mut g, &FILLER, 0);
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx| {
        // Life and Limb makes Raphael 1/1. Put real counters on him so the damage witness survives.
        cx.acting_as(PlayerId(1), |cx| cantrip_api::Game::add_counters(cx, source, CounterKind::PlusOne, 2));
        cx.acting_as(PlayerId(0), |cx| cantrip_api::Game::damage(cx, dealer, Target::Object(source), 2));
    });
    assert_eq!(g.state.triggers.len(), 1);
    assert_eq!(g.state.triggers[0].data[0], 2);
    let expected = (ColorSet::of(Color::Green).with(Color::Red), false);
    assert_capability_isolated(&mut g, source, expected);
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], crate::settle::settle).unwrap();
    let item = *g.state.stack.last().unwrap();
    assert_eq!(item.data[0], 2);
    assert_capability_isolated(&mut g, source, expected);
    in_frame(&mut g, cantrip_state::FrameKind::Resolve { serial: item.serial }, vec![], crate::resolve::resolve).unwrap();
    assert_eq!(g.state.player(PlayerId(1)).pool.get(Mana::R), 2);
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green), false));
}

fn chance_colors(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let choice = if g.roll_die(item.controller, 2).natural == 1 { Color::Green } else { Color::Red };
    let Answer::Colors(colors) = g.ask(item.controller, RequestKind::ChooseColors {
        source: item.object, choices: ColorSet::of(choice), count: 1, purpose: ColorUse::AddMana,
    })? else { unreachable!() };
    assert_eq!(colors, [choice], "a chance branch must announce its own later questions");
    g.add_mana(item.controller, choice.mana(), 1);
    if g.roll_die(item.controller, 20).natural == 20 { g.add_mana(item.controller, Mana::W, 1); }
    let coin = if g.flip_coin(item.controller) { Mana::U } else { Mana::B };
    g.add_mana(item.controller, coin, 1);
    if g.choose_at_random(31) == 30 { g.add_mana(item.controller, Mana::C, 1); }
    Ok(())
}

#[test]
fn hypothetical_chance_replays_all_faces_coins_and_late_random_indices_without_consuming_real_randomness() {
    static SOURCE: CardDef = CardDef { name: "Capability chance and question witness",
        abilities: &[ability(Cost::NONE, chance_colors, "Roll and flip; conditionally add each type.").produces_mana()], ..FILLER };
    let mut g = choice_game([&FILLER, &SOURCE]);
    let source = put(&mut g, &SOURCE, 1);
    g.state.force_rolls(&[1, 1, 1]);
    assert_capability_isolated(&mut g, source, (ColorSet(0b11111), true));
}

fn chosen_order_mana(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let cards = g.state().zone(Zone::graveyard(item.controller)).to_vec();
    let Answer::Cards(chosen) = g.ask(item.controller, RequestKind::ChooseCards {
        source: item.object, purpose: Purpose::ReturnToHand, cards, rule: Box::new(Selection::new(Count::Exactly(2))),
    })? else { unreachable!() };
    if g.chars(chosen[0]).colors.has(Color::Green) && g.chars(chosen[1]).colors.has(Color::Red) {
        g.add_mana(item.controller, Mana::W, 1);
    }
    Ok(())
}

#[test]
fn hypothetical_card_selections_keep_order_and_use_the_real_selection_rule() {
    static SOURCE: CardDef = CardDef { name: "Capability ordered card selection",
        abilities: &[ability(Cost::NONE, chosen_order_mana, "Choose two cards; conditionally add white.").produces_mana()], ..FILLER };
    let mut g = choice_game([&FILLER, &SOURCE, &GREEN_TARGET, &RED_TARGET]);
    let source = put(&mut g, &SOURCE, 1);
    add_object(&mut g, &GREEN_TARGET, PlayerId(1), Zone::graveyard(PlayerId(1)));
    assert_capability_isolated(&mut g, source, (ColorSet::NONE, false));
    add_object(&mut g, &RED_TARGET, PlayerId(1), Zone::graveyard(PlayerId(1)));
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::White), false));
}


#[test]
fn actual_name_sticker_goblin_and_shared_firebending_keep_their_ordinary_trigger_classification() {
    let mut g = printed_game();
    let source = printed_creature_as_forest(&mut g, "\"Name Sticker\" Goblin", Subtype::Goblin);
    let goblin = printed(&g, "\"Name Sticker\" Goblin");
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green).with(Color::Red), false));
    for _ in 0..8 { put(&mut g, goblin, 1); }
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green).with(Color::Red), false));
    put(&mut g, goblin, 1);
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green), false));
    let mut g = printed_game();
    let source = printed_creature_as_forest(&mut g, "Firebending Student", Subtype::Human);
    assert!(g.chars(source).abilities().any(|a| matches!(g.cards.ability(a), AbilityDef::Triggered(d) if d.produces_mana && !d.mana_ability)));
    assert!(g.state.triggers.is_empty());
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green).with(Color::Red), false));
}


// Preserved rules-correct current printed counterexample, deliberately not ignored or changed to expect a panic.
#[test]
fn actual_wizards_rockets_includes_unbounded_announced_x_without_requiring_any_payment() {
    let mut g = printed_game();
    let def = printed(&g, "Wizard's Rockets");
    let source = put(&mut g, def, 1);
    g.state.update_obj(source, |o| o.tapped = true);
    assert_eq!(g.state.obj(source).x, 0);
    assert!(g.state.player(PlayerId(1)).pool.is_empty());
    assert_capability_isolated(&mut g, source, (ColorSet(0b11111), false));
}

fn rockets_as_land(g: &mut Game, source: ObjectId) {
    in_frame(g, cantrip_state::FrameKind::Settle, vec![], |cx| {
        cx.acting_as(PlayerId(1), |cx| cantrip_api::Game::add_effect(cx, source, source,
            EffectKind::AddTypes(TypeSet::of(&[CardType::Land])), Duration::EndOfTurn));
    });
    assert!(g.chars(source).is(CardType::Land));
}

fn activate_rockets(g: &mut Game, source: ObjectId, x: u32) {
    let own_x = g.state.obj(source).x;
    g.state.add_mana(PlayerId(1), Mana::C, x, ObjectId::NONE, None, ManaLasts::Step, false);
    // One color choice for all X mana (CR 106.1b): the first 255 blue and the rest red.
    let mut answers = vec![Answer::Announce(x)];
    if x > 0 {
        answers.push(Answer::Colors((0..x).map(|i| if i < 255 { Color::Blue } else { Color::Red }).collect()));
    }
    in_frame(g, cantrip_state::FrameKind::Settle, answers, |cx| {
        cx.acting_as(PlayerId(1), |cx| crate::actions::activate_mana(cx, PlayerId(1), source, 0, None, false, &[]))
    }).unwrap();
    assert!(g.state.obj(source).became.is_some(), "the actual sacrifice cost is paid");
    assert_eq!(g.state.obj(source).x, own_x, "the activation's X is local and leaves the source's own X alone (CR 107.3a)");
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::AbilityActivated { source: id, .. } if id == source)).count(), 1);
}

#[test]
fn actual_wizards_rockets_announces_pays_sacrifices_and_produces_all_300_mana() {
    let mut g = printed_game();
    let rockets = printed(&g, "Wizard's Rockets");
    let source = put(&mut g, rockets, 1);
    let sphere = printed(&g, "Damping Sphere");
    put(&mut g, sphere, 0);
    assert_capability_isolated(&mut g, source, (ColorSet(0b11111), false));
    activate_rockets(&mut g, source, 300);
    assert_eq!(g.state.player(PlayerId(1)).pool.0, [0, 255, 0, 45, 0, 0]);
    assert_eq!(g.state.player(PlayerId(1)).pool.total(), 300);
    assert!(g.log.iter().any(|r| matches!(r.event, Event::TappedForMana { permanent, produced, .. }
        if permanent == source && produced.0 == [0, 255, 0, 45, 0, 0])));
    let mut g = printed_game();
    let def = printed(&g, "Wizard's Rockets");
    let source = put(&mut g, def, 1);
    activate_rockets(&mut g, source, 0);
    assert!(g.state.player(PlayerId(1)).pool.is_empty());
    assert!(g.log.iter().any(|r| matches!(r.event, Event::TappedForMana { permanent, produced, .. }
        if permanent == source && produced.is_empty())), "CR 106.12: activating a mana ability with T taps for mana even at X=0");
    let mut g = printed_game();
    let rockets = printed(&g, "Wizard's Rockets");
    let source = put(&mut g, rockets, 1);
    activate_rockets(&mut g, source, 1024);
    assert_eq!(g.state.player(PlayerId(1)).pool, ManaPool([0, 255, 0, 769, 0, 0]), "no arbitrary 1000 announcement cap");
}

#[test]
fn actual_damping_sphere_uses_total_threshold_nonland_refusal_and_sacrificed_land_lki() {
    for x in [0, 1, 2, 300] {
        let mut g = printed_game();
        let rockets = printed(&g, "Wizard's Rockets");
        let source = put(&mut g, rockets, 1);
        let sphere = printed(&g, "Damping Sphere");
        put(&mut g, sphere, 0);
        rockets_as_land(&mut g, source);
        assert_capability_isolated(&mut g, source, (ColorSet(0b11111), true));
        activate_rockets(&mut g, source, x);
        let expected = match x { 0 => ManaPool::default(), 1 => ManaPool([0, 1, 0, 0, 0, 0]),
            _ => ManaPool([0, 0, 0, 0, 0, 1]) };
        assert_eq!(g.state.player(PlayerId(1)).pool, expected, "actual production at X={x}");
        assert!(g.chars(source).is(CardType::Land), "sacrificed source uses its land LKI");
    }
    let mut g = printed_game();
    let sphere = printed(&g, "Damping Sphere");
    put(&mut g, sphere, 0);
    let tomb = printed(&g, "Ancient Tomb");
    let source = put(&mut g, tomb, 1);
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx|
        cx.acting_as(PlayerId(1), |cx| crate::actions::activate_mana(cx, PlayerId(1), source, 0, None, false, &[]))).unwrap();
    assert_eq!(g.state.player(PlayerId(1)).pool, ManaPool([0, 0, 0, 0, 0, 1]));
    assert_eq!(g.state.player(PlayerId(1)).life, 18, "Ancient Tomb's nonmana effect is preserved");
}

#[test]
fn unbounded_combination_partitions_preserve_each_replacement_order_and_optional_decline() {
    let mut g = printed_game();
    let rockets = printed(&g, "Wizard's Rockets");
    let source = put(&mut g, rockets, 1);
    rockets_as_land(&mut g, source);
    let sphere = printed(&g, "Damping Sphere");
    put(&mut g, sphere, 0);
    put(&mut g, &CONTAMINATION, 0);
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Black), false));
    // A second threshold-one fixed-output peer makes the order observable: the last
    // replacement wins. Use the same core declaration with a different color.
    static WHITE: CardDef = CardDef { name: "Capability fixed white peer",
        replacements: &[fixed_mana_replacement(1, land_source, Mana::W, 1, "white instead")], ..CardDef::BLANK };
    static SOURCE: CardDef = CardDef { name: "Capability declared X land",
        abilities: &[mana_ability(cost("{X}, {T}"), cantrip_mechanics::mana::announced_x_in_any_combination::<0b11111>(), "Add X mana.")], ..FILLER };
    let mut g = choice_game([&FILLER, &SOURCE, &WHITE, &CONTAMINATION]);
    let source = put(&mut g, &SOURCE, 1);
    put(&mut g, &WHITE, 0);
    put(&mut g, &CONTAMINATION, 0);
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::White).with(Color::Black), false));
    static OPTIONAL: CardDef = CardDef { name: "Capability optional fixed white",
        replacements: &[fixed_mana_replacement(1, land_source, Mana::W, 1, "white instead")
            .optional(&Question::new("replace", "Replace the mana?", "", "Yes", "No"))], ..CardDef::BLANK };
    let mut g = choice_game([&FILLER, &SOURCE, &OPTIONAL]);
    let source = put(&mut g, &SOURCE, 1);
    put(&mut g, &OPTIONAL, 0);
    assert_capability_isolated(&mut g, source, (ColorSet(0b11111), false));
    static FAR_THRESHOLD: CardDef = CardDef { name: "Capability printed threshold family",
        replacements: &[fixed_mana_replacement(1_000_001, land_source, Mana::C, 1, "colorless instead")], ..CardDef::BLANK };
    let mut g = choice_game([&FILLER, &SOURCE, &FAR_THRESHOLD]);
    let source = put(&mut g, &SOURCE, 1);
    put(&mut g, &FAR_THRESHOLD, 0);
    assert_capability_isolated(&mut g, source, (ColorSet(0b11111), true), /* threshold directly defines an interval */);
}

/// A replacement without a declared contract is judged at one and at two mana, and never panics: the regression of
/// an `expect` that a real game reached (Wizard's Rockets made a land, under Ritual of Subdual or Deep Water).
#[test]
fn an_opaque_replacement_on_an_unbounded_combination_is_judged_without_a_contract() {
    static SOURCE: CardDef = CardDef { name: "Capability opaque replacement X land",
        abilities: &[mana_ability(cost("{X}, {T}"), cantrip_mechanics::mana::announced_x_in_any_combination::<0b11111>(), "Add X mana.")], ..FILLER };
    static OPAQUE: CardDef = CardDef { name: "Capability opaque mana replacement",
        replacements: &[replacement(ReplacementKind::ManaProduction, land_tapped, black_instead, "black instead")], ..CardDef::BLANK };
    let mut g = choice_game([&FILLER, &SOURCE, &OPAQUE]);
    let source = put(&mut g, &SOURCE, 1);
    put(&mut g, &OPAQUE, 0);
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Black), false));
}

/// Wizard's Rockets' recipe on a land under the printed Ritual of Subdual (an undeclared printed replacement) and Deep
/// Water's effect (an undeclared granted one): the query once hit an `expect` and an `assert` on each, a crash a real
/// game reaches (March of the Machines, Arcane Adaptation naming Saproling and Life and Limb make Wizard's Rockets a
/// Forest; an opponent's Exotic Orchard asks). Every type becomes colorless, or blue.
#[test]
fn printed_undeclared_mana_replacements_judge_an_announced_x_combination() {
    static SOURCE: CardDef = CardDef { name: "Capability Rockets-recipe land",
        abilities: &[mana_ability(cost("{X}, {T}"), cantrip_mechanics::mana::announced_x_in_any_combination::<0b11111>(), "Add X mana in any combination of colors.")], ..FILLER };
    let ritual = cantrip_cards::all().into_iter().find(|d| d.name == "Ritual of Subdual").unwrap();
    let deep = cantrip_cards::all().into_iter().find(|d| d.name == "Deep Water").unwrap();
    let mut g = choice_game([&FILLER, &SOURCE, ritual, deep]);
    let source = put(&mut g, &SOURCE, 1);
    assert_capability_isolated(&mut g, source, (ColorSet(0b11111), false));
    let ritual = put(&mut g, ritual, 0);
    assert_capability_isolated(&mut g, source, (ColorSet::NONE, true));
    g.state.zone_remove(Zone::BATTLEFIELD, ritual);
    g.state.update_obj(ritual, |o| o.zone = Zone::graveyard(PlayerId(0)));
    g.state.zone_push(Zone::graveyard(PlayerId(0)), ritual);
    g.state.see_statics();
    assert_capability_isolated(&mut g, source, (ColorSet(0b11111), false));
    // Deep Water's activation, resolved for the land's controller: "if you tap a land you control for mana".
    let water = put(&mut g, deep, 1);
    let AbilityDef::Activated(ability) = deep.abilities[0] else { unreachable!() };
    let item = crate::actions::stack_item(StackKind::Activated, water, AbilityRef { card: g.cards.id_of(deep), index: 0 }, PlayerId(1), &[]);
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx| cx.acting_as(PlayerId(1), |cx| (ability.effect)(cx, &item))).unwrap();
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Blue), false));
}

/// A question with more answers than are tried ends the judgment without a hang or a panic: "exile any number of
/// target cards from your graveyard" over 40 cards has 2^40 sets of targets. What was found stands (KI-124).
#[test]
fn an_ability_with_too_many_answers_is_judged_in_bounded_time() {
    fn exile_any_number(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
        let n = g.state().targets(item).iter().filter(|t| t.object().is_some()).count() as u32;
        g.add_mana(item.controller, Mana::R, n);
        Ok(())
    }
    static SOURCE: CardDef = CardDef { name: "Capability any number of targets",
        abilities: &[ability(loyalty(-6), exile_any_number, "Exile any number of target cards from your graveyard. Add {R} for each.")
            .targets(&[up_to(TargetSpec::ANY_NUMBER, grave_land)]).produces_mana()], ..FILLER };
    let mut g = choice_game([&FILLER, &SOURCE, &GREEN_TARGET]);
    for _ in 0..40 {
        add_object(&mut g, &GREEN_TARGET, PlayerId(1), Zone::graveyard(PlayerId(1)));
    }
    let source = put(&mut g, &SOURCE, 1);
    let judged = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| g.producible_types(source)));
    if crate::layers::verifying() {
        let error = judged.expect_err("a gap is found under verification");
        let text = error.downcast_ref::<String>().map(String::as_str).or_else(|| error.downcast_ref::<&str>().copied()).unwrap();
        assert!(text.contains("answers"), "{text}");
    } else {
        assert_eq!(judged.expect("no panic in a game"), (ColorSet::of(Color::Red), false));
    }
    assert!(g.producible_path.borrow().is_empty());
}

#[test]
fn current_vivi_and_desolation_definitions_prove_the_printed_combination_recipe_at_bounded_counts() {
    let mut g = printed_game();
    let source = printed_creature_as_forest(&mut g, "Vivi Ornitier", Subtype::Wizard);
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx|
        cx.acting_as(PlayerId(1), |cx| cantrip_api::Game::add_counters(cx, source, CounterKind::PlusOne, 2)));
    assert_eq!(g.chars(source).power, Some(3));
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green).with(Color::Blue).with(Color::Red), false));
    let def = printed(&g, "Vivi Ornitier");
    let AbilityDef::Activated(ability) = def.abilities[0] else { unreachable!() };
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![Answer::Colors(vec![Color::Blue, Color::Red, Color::Blue])], |cx|
        cx.acting_as(PlayerId(1), |cx| ability.mana.unwrap()(cx, &Asker::new(source, PlayerId(1)), None))).unwrap();
    assert_eq!(g.state.player(PlayerId(1)).pool, ManaPool([0, 2, 0, 1, 0, 0]));
    let mut g = printed_game();
    let def = printed(&g, "Desolation of Smaug");
    let source = add_object(&mut g, def, PlayerId(1), Zone::STACK);
    let item = crate::actions::stack_item(StackKind::Spell, source, AbilityRef::spell(g.cards.id_of(def)), PlayerId(1), &[]);
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![Answer::Colors(vec![Color::White, Color::Blue, Color::White, Color::Red])], |cx|
        cx.acting_as(PlayerId(1), |cx| (def.spell.unwrap().effect)(cx, &item))).unwrap();
    assert_eq!(g.state.restricted_mana.len(), 1);
    assert_eq!(g.state.restricted_mana[0].pool, ManaPool([2, 1, 0, 1, 0, 0]));
}

// The original printed caller regression stays enabled with its rules-correct expectation.
#[test]
fn actual_vivi_as_forest_at_power_256_still_contributes_its_printed_blue_and_red() {
    let mut g = printed_game();
    let source = printed_creature_as_forest(&mut g, "Vivi Ornitier", Subtype::Wizard);
    in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx|
        cx.acting_as(PlayerId(1), |cx| cantrip_api::Game::add_counters(cx, source, CounterKind::PlusOne, 255)));
    assert_eq!(g.chars(source).power, Some(256));
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green).with(Color::Blue).with(Color::Red), false));
}

fn vivi_offered(g: &mut Game, source: ObjectId) -> bool {
    in_frame(g, cantrip_state::FrameKind::Settle, vec![], |cx|
        crate::actions::priority_options(cx, PlayerId(1)).iter().any(|o| o.source == Some(source) && o.index == 0
            && o.kind == ActionKind::ActivateAbility))
}

#[test]
fn actual_vivi_full_width_current_power_choices_and_real_activation_limits() {
    for power in [-1, 0, 1, 255, 256, 300] {
        for strategy in 0..3 {
            let mut g = printed_game();
            g.state.update_turn(|t| { t.number = 2; t.active = PlayerId(1); t.priority = PlayerId(1); t.set_step(Step::PrecombatMain); });
            g.state.update_player(PlayerId(1), |p| p.turn_began = 2);
            let vivi = printed(&g, "Vivi Ornitier");
            let source = put(&mut g, vivi, 1);
            let base = g.chars(source).power.unwrap();
            in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx|
                cx.acting_as(PlayerId(1), |cx| cantrip_api::Game::add_effect(cx, source, source,
                    EffectKind::Pump(power - base, 0), Duration::EndOfTurn)));
            assert_eq!(g.chars(source).power, Some(power), "the current layered power determines the amount");
            let expected_colors = if power > 0 { ColorSet::of(Color::Blue).with(Color::Red) } else { ColorSet::NONE };
            assert_capability_isolated(&mut g, source, (expected_colors, false));
            assert!(vivi_offered(&mut g, source));
            let count = power.max(0) as usize;
            let colors: Vec<_> = (0..count).map(|i| match strategy { 0 => Color::Blue, 1 => Color::Red,
                _ if i % 2 == 0 => Color::Blue, _ => Color::Red }).collect();
            let mut answers = vec![Answer::Act(Action::of(&ActionOption::on(ActionKind::ActivateAbility, source, 0)))];
            answers.extend((!colors.is_empty()).then(|| Answer::Colors(colors.clone())));
            in_frame(&mut g, cantrip_state::FrameKind::Settle, answers, |cx| crate::actions::act(cx, PlayerId(1))).unwrap();
            let blue = colors.iter().filter(|&&c| c == Color::Blue).count() as u32;
            let red = count as u32 - blue;
            assert_eq!(g.state.player(PlayerId(1)).pool, ManaPool([0, blue, 0, red, 0, 0]), "power {power}, strategy {strategy}");
            assert!(!vivi_offered(&mut g, source), "the real once-per-turn limit applies even to zero production");
            assert_capability_isolated(&mut g, source, (expected_colors, false));
            g.state.update_turn(|t| { t.number = 3; t.active = PlayerId(0); t.priority = PlayerId(1); });
            assert!(!vivi_offered(&mut g, source), "the real your-turn condition applies");
            g.state.update_turn(|t| { t.number = 4; t.active = PlayerId(1); });
            assert!(vivi_offered(&mut g, source), "a later own turn permits a fresh activation");
        }
    }
}

#[test]
fn actual_damping_and_a_second_fixed_replacement_apply_in_each_chosen_order_after_sacrifice() {
    static TWO_WHITE: CardDef = CardDef { name: "Capability fixed two white",
        replacements: &[fixed_mana_replacement(1, land_source, Mana::W, 2, "two white instead")], ..CardDef::BLANK };
    let sphere = cantrip_cards::all().into_iter().find(|d| d.name == "Damping Sphere").unwrap();
    let rockets = cantrip_cards::all().into_iter().find(|d| d.name == "Wizard's Rockets").unwrap();
    for first in [0, 1] {
        let mut g = choice_game([&FILLER, rockets, sphere, &TWO_WHITE]);
        let source = put(&mut g, rockets, 1);
        rockets_as_land(&mut g, source);
        put(&mut g, sphere, 0);
        put(&mut g, &TWO_WHITE, 0);
        assert_capability_isolated(&mut g, source, (ColorSet::of(Color::White), true));
        g.state.add_mana(PlayerId(1), Mana::C, 2, ObjectId::NONE, None, ManaLasts::Step, false);
        in_frame(&mut g, cantrip_state::FrameKind::Settle,
            vec![Answer::Announce(2), Answer::Colors(vec![Color::Red, Color::Red]), Answer::Index(first)], |cx|
            cx.acting_as(PlayerId(1), |cx| crate::actions::activate_mana(cx, PlayerId(1), source, 0, None, false, &[]))).unwrap();
        assert_eq!(g.state.player(PlayerId(1)).pool,
            if first == 0 { ManaPool([2, 0, 0, 0, 0, 0]) } else { ManaPool([0, 0, 0, 0, 0, 1]) },
            "each effect applies only once; later guards inspect the reshaped quantity");
        assert!(g.state.obj(source).became.is_some());
        assert!(g.chars(source).is(CardType::Land));
    }
}

#[test]
fn integrated_commander_authority_supplies_actual_signet_colors_through_an_orchard_advice_copy() {
    let signet = cantrip_cards::all().into_iter().find(|d| d.name == "Arcane Signet").unwrap();
    let vivi = cantrip_cards::all().into_iter().find(|d| d.name == "Vivi Ornitier").unwrap();
    let cards = Box::leak(Box::new(CardTable::new([&FILLER, &ORCHARD, signet, vivi])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&FILLER); 20]; 2],
        commander: true, commanders: vec![vec![], vec![cards.id_of(vivi)]], ..Default::default() });
    let source = put(&mut g, signet, 1);
    rockets_as_land(&mut g, source);
    let colors = ColorSet::of(Color::Blue).with(Color::Red);
    assert_capability_isolated(&mut g, source, (colors, false));
    let orchard = put(&mut g, &ORCHARD, 0);
    assert_capability_isolated(&mut g, orchard, (colors, false));
    assert!(g.state.objects.iter().filter(|o| o.commander).all(|o| o.zone.is(ZoneKind::Command)),
        "a commander's physical designation and identity survive every advice query");
}

// Coalition Relic's printed count remains full width; an existing u8 cast must not erase it.
#[test]
fn actual_coalition_relic_releases_every_charge_counter_above_the_wire_count() {
    for count in [0u16, 1, 255, 256, 300] {
        let mut g = printed_game();
        let relic = printed(&g, "Coalition Relic");
        let source = put(&mut g, relic, 1);
        in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx|
            cx.acting_as(PlayerId(1), |cx| cantrip_api::Game::add_counters(cx, source, CounterKind::Charge, i32::from(count))));
        assert_eq!(g.state.obj(source).counters(CounterKind::Charge), i32::from(count), "full-width setup");
        let colors: Vec<_> = (0..count).map(|i| if i % 2 == 0 { Color::Blue } else { Color::Red }).collect();
        let answers = (!colors.is_empty()).then(|| Answer::Colors(colors.clone())).into_iter().collect();
        let AbilityDef::Triggered(trigger) = relic.abilities[2] else { unreachable!() };
        let item = crate::actions::stack_item(StackKind::Triggered, source,
            AbilityRef { card: g.cards.id_of(relic), index: 2 }, PlayerId(1), &[]);
        assert!(!trigger.mana_ability, "the release retains ordinary stack timing");
        in_frame(&mut g, cantrip_state::FrameKind::Settle, answers, |cx| {
            cx.st().stack_push(item);
            let serial = cx.s().stack.last().unwrap().serial;
            let frame = cx.frame;
            cx.st().update_frame(frame, |f| f.kind = cantrip_state::FrameKind::Resolve { serial });
            cx.acting_as(PlayerId(1), crate::resolve::resolve)
        }).unwrap();
        assert_eq!(g.state.obj(source).counters(CounterKind::Charge), 0, "all counters come off");
        let blue = (u32::from(count) + 1) / 2;
        let red = u32::from(count) / 2;
        assert_eq!(g.state.player(PlayerId(1)).pool, ManaPool([0, blue, 0, red, 0, 0]),
            "each removed charge counter adds exactly one mana, count {count}");
    }
}


#[test]
fn coalition_terminal_release_alone_has_full_width_capability_without_removing_real_counters() {
    for count in [0, 1, 255, 256, 300, 65_535, 65_536, 100_000] {
        let mut g = printed_game();
        let relic = printed(&g, "Coalition Relic");
        let source = put(&mut g, relic, 1);
        rockets_as_land(&mut g, source);
        let release = AbilityRef { card: g.cards.id_of(relic), index: 2 };
        in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx| {
            cx.acting_as(PlayerId(1), |cx| {
                // Isolate the printed release: the direct any-color activation cannot mask it.
                cantrip_api::Game::add_effect(cx, source, source, EffectKind::LoseAllAbilities, Duration::EndOfTurn);
                cantrip_api::Game::add_effect(cx, source, source, EffectKind::AddAbility(release), Duration::EndOfTurn);
                cantrip_api::Game::add_counters(cx, source, CounterKind::Charge, count);
                cantrip_api::Game::add_counters(cx, source, CounterKind::PlusOne, 2);
            });
        });
        assert_eq!(g.chars(source).abilities().collect::<Vec<_>>(), vec![release], "only the release is queried");
        g.state.update_obj(source, |o| o.tapped = true);
        let expected = if count == 0 { ColorSet::NONE } else { ColorSet(0b11111) };
        assert_capability_isolated(&mut g, source, (expected, false));
        let orchard = put(&mut g, &ORCHARD, 0);
        assert_capability_isolated(&mut g, orchard, (expected, false));
        put(&mut g, &CONTAMINATION, 0);
        // A phase trigger never taps the source for mana, so Contamination cannot replace it.
        assert_capability_isolated(&mut g, source, (expected, false));
        let orchard_expected = if count == 0 { ColorSet::NONE } else { ColorSet::of(Color::Black) };
        assert_capability_isolated(&mut g, orchard, (orchard_expected, false));
        assert_eq!(g.state.obj(source).counters(CounterKind::Charge), count);
        assert_eq!(g.state.obj(source).counters(CounterKind::PlusOne), 2, "unrelated counters remain");
    }
}

fn choose_then_remove(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<u32, Suspend> {
    let Answer::Colors(colors) = g.ask(item.controller, RequestKind::ChooseColors {
        source: item.object, choices: ColorSet(0b01010), count: 1, purpose: ColorUse::AddMana,
    })? else { unreachable!() };
    let remove = if colors[0] == Color::Blue { 1 } else { 0 };
    Ok(g.remove_counters(item.object, CounterKind::Charge, remove).max(0) as u32)
}

#[test]
fn terminal_combination_runs_preparation_choices_and_intervening_conditions() {
    static CHOICE: CardDef = CardDef { name: "Capability chosen removal then terminal green",
        abilities: &[triggered_mana_combination(cantrip_mechanics::triggers::YOUR_FIRST_MAIN_PHASE,
            choose_then_remove, ColorSet::of(Color::Green), "Choose a color; remove a charge if blue; add green per removed counter.")],
        ..FILLER };
    static CONDITIONAL: CardDef = CardDef { name: "Capability conditional terminal green",
        abilities: &[triggered_mana_combination(cantrip_mechanics::triggers::YOUR_FIRST_MAIN_PHASE,
            choose_then_remove, ColorSet::of(Color::Green), "If tapped, choose and remove then add green.")
            .condition(|q, a| q.obj(a.source).tapped)], ..FILLER };
    let cards = Box::leak(Box::new(CardTable::new([&FILLER, &CHOICE, &CONDITIONAL])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&FILLER); 20]; 2], ..Default::default() });
    for def in [&CHOICE, &CONDITIONAL] {
        let source = put(&mut g, def, 1);
        assert_capability_isolated(&mut g, source, (ColorSet::NONE, false));
        in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx|
            cx.acting_as(PlayerId(1), |cx| cantrip_api::Game::add_counters(cx, source, CounterKind::Charge, 300)));
        let expected = if std::ptr::eq(def, &CONDITIONAL) { ColorSet::NONE } else { ColorSet::of(Color::Green) };
        assert_capability_isolated(&mut g, source, (expected, false));
        g.state.update_obj(source, |o| o.tapped = true);
        assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green), false));
        assert_eq!(g.state.obj(source).counters(CounterKind::Charge), 300);
    }
}

fn opaque_after_combination(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::mana::add_in_any_combination(g, &Asker::new(item.object, item.controller), ColorSet(0b01010), 1)?;
    if g.state().player(item.controller).pool.get(Mana::U) > 0 {
        g.add_mana(item.controller, Mana::G, 1);
    }
    Ok(())
}

#[test]
fn an_opaque_callback_after_combination_is_executed_with_the_actual_hypothetical_pool() {
    static OPAQUE: CardDef = CardDef { name: "Capability opaque callback after color choice",
        abilities: &[triggered(cantrip_mechanics::triggers::YOUR_FIRST_MAIN_PHASE,
            opaque_after_combination, "Add blue or red, then green if the pool has blue.").produces_mana()], ..FILLER };
    let cards = Box::leak(Box::new(CardTable::new([&FILLER, &OPAQUE])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&FILLER); 20]; 2], ..Default::default() });
    let source = put(&mut g, &OPAQUE, 1);
    assert_capability_isolated(&mut g, source, (ColorSet(0b11010), false));
}

// Reviewer counterexample, with a positive control using the chance-aware Game API.
// Both abilities can produce G or R. Only the order of two known library cards matters;
// no seed is searched, and whichever one ordinary RNG picks cannot prove the other type.
fn random_bottom_mana(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let library = g.state().zone(Zone::library(item.controller)).to_vec();
    let pair = &library[library.len()-2..];
    g.bottom_in_random_order(item.controller, pair);
    let bottom = *g.state().zone(Zone::library(item.controller)).last().unwrap();
    let mana = if g.chars(bottom).colors.has(Color::Green) { Mana::G } else { Mana::R };
    g.add_mana(item.controller, mana, 1);
    Ok(())
}

fn direct_random_mana_control(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let mana = if g.random(2) == 0 { Mana::G } else { Mana::R };
    g.add_mana(item.controller, mana, 1);
    Ok(())
}

#[test]
fn random_bottom_order_enumerates_all_mana_types() {
    static BOTTOM: CardDef = CardDef { name: "Reviewer random bottom mana producer",
        abilities: &[ability(Cost::NONE, random_bottom_mana,
            "Randomize two library cards; add the bottom card's color.").produces_mana()], ..FILLER };
    static DIRECT: CardDef = CardDef { name: "Reviewer direct random positive control",
        abilities: &[ability(Cost::NONE, direct_random_mana_control,
            "Randomly add green or red.").produces_mana()], ..FILLER };
    let mut g = choice_game([&FILLER, &BOTTOM, &DIRECT, &GREEN_TARGET, &RED_TARGET]);
    let direct = put(&mut g, &DIRECT, 1);
    let expected = (ColorSet::of(Color::Green).with(Color::Red), false);
    assert_capability_isolated(&mut g, direct, expected);
    let source = put(&mut g, &BOTTOM, 1);
    add_object(&mut g, &GREEN_TARGET, PlayerId(1), Zone::library(PlayerId(1)));
    add_object(&mut g, &RED_TARGET, PlayerId(1), Zone::library(PlayerId(1)));
    let library = g.state.zone(Zone::library(PlayerId(1))).to_vec();
    let last = &library[library.len()-2..];
    assert!(g.chars(last[0]).colors.has(Color::Green));
    assert!(g.chars(last[1]).colors.has(Color::Red));
    assert_eq!(g.producible_types(source), expected,
        "reviewer: random bottom order must include both possible produced mana colors");
    assert_capability_isolated(&mut g, source, expected);
}


fn random_bottom_placement_mana(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let library = g.state().zone(Zone::library(item.controller)).to_vec();
    let pair = &library[library.len()-2..];
    g.put_into_library(pair, LibraryPlace::BottomInRandomOrder);
    let bottom = *g.state().zone(Zone::library(item.controller)).last().unwrap();
    let mana = if g.chars(bottom).colors.has(Color::Green) { Mana::G } else { Mana::R };
    g.add_mana(item.controller, mana, 1);
    Ok(())
}

// CR 106.7/401.4: the library-placement verb shares the same exhaustive chance authority.
#[test]
fn random_bottom_library_placement_enumerates_all_mana_types() {
    static SOURCE: CardDef = CardDef { name: "Random library placement mana producer",
        abilities: &[ability(Cost::NONE, random_bottom_placement_mana,
            "Put two library cards on the bottom at random; add the bottom card's color.").produces_mana()], ..FILLER };
    let mut g = choice_game([&FILLER, &SOURCE, &GREEN_TARGET, &RED_TARGET]);
    let source = put(&mut g, &SOURCE, 1);
    add_object(&mut g, &GREEN_TARGET, PlayerId(1), Zone::library(PlayerId(1)));
    add_object(&mut g, &RED_TARGET, PlayerId(1), Zone::library(PlayerId(1)));
    assert_capability_isolated(&mut g, source, (ColorSet::of(Color::Green).with(Color::Red), false));
}

// Ordinary play retains its generator and exact randomized events. Reversing a fixed input
// is a deterministic control, not a search through seeds or random game positions.
#[test]
fn actual_random_bottom_order_preserves_rng_and_randomized_event_contract() {
    for reverse in [false, true] {
        let mut g = choice_game([&FILLER, &GREEN_TARGET, &RED_TARGET]);
        let player = PlayerId(1);
        let zone = Zone::library(player);
        let green = add_object(&mut g, &GREEN_TARGET, player, zone);
        let red = add_object(&mut g, &RED_TARGET, player, zone);
        let pair = if reverse { vec![red, green] } else { vec![green, red] };
        let mut expected = pair.clone();
        let mut ordinary = g.state.clone();
        let index = ordinary.random(2) as usize;
        expected.swap(1, index);
        let mut expected_library = g.state.zone(zone).to_vec();
        expected_library.retain(|id| !pair.contains(id));
        expected_library.extend(expected.iter().copied());
        let history = g.log.len();
        in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx| {
            cantrip_api::Game::bottom_in_random_order(cx, player, &pair);
        });
        assert_eq!(g.state.rng, ordinary.rng, "ordinary RNG advances exactly once for a two-card order");
        assert_eq!(g.state.zone(zone), expected_library.as_slice(), "ordinary order uses the same RNG value");
        let events: Vec<_> = g.log[history..].iter().filter_map(|record| match record.event {
            Event::Randomized { player: actor, card } if actor == player => Some(card),
            _ => None,
        }).collect();
        assert_eq!(events, expected, "every randomized card is recorded in actual order");
        let rng = g.state.rng;
        in_frame(&mut g, cantrip_state::FrameKind::Settle, vec![], |cx| {
            assert_eq!(crate::ops::random_below(cx, 0), 0);
            assert_eq!(crate::ops::random_below(cx, 1), 0);
        });
        assert_eq!(g.state.rng, rng, "an empty or singleton choice consumes no real random value");
        assert!(g.state.player(player).pool.is_empty(), "ordering does not itself add mana");
    }
}
