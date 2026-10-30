//! A cast is the card's own option (its face, its own cost) and what made it possible (`ActionOption::granted_by`, with
//! `paid_by` for an alternative cost another object gives): every permission that would let a card be cast is an option of
//! its own, and the cast is made by the one chosen, on its terms (CR 601.2b, 601.3, 118.9). The cards here are small
//! probes for the permission machinery, not card implementations. Taken, as rules, from the tests of pull request #147.

use std::sync::OnceLock;

use cantrip_api::*;
use cantrip_rules::{Game as RulesGame, Setup};
use cantrip_state::{Frame, FrameKind};

const ME: PlayerId = PlayerId(0);

static PROBE_SPELL: CardDef = CardDef {
    name: "Route Probe",
    oracle: true,
    cost: ManaCost { generic: 2, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    spell: Some(SpellDef::MODAL),
    ..CardDef::BLANK
};

/// An alternative cost that names no zone (Force of Will's): open from wherever a permission has the card cast.
static HAND_WAY: WayDef = WayDef {
    text: "hand-named cost",
    cost: Cost::mana(ManaCost::ZERO),
    from: ZoneKind::Hand,
    only_from: false,
    exile_after: false,
    condition: None,
    instant_speed: true,
    enters_attacking: false,
    kind: WayKind::Other,
    turn_up: Cost::NONE,
};

/// One that casts it from its owner's graveyard only.
static GRAVEYARD_WAY: WayDef = WayDef {
    text: "graveyard alternate way",
    cost: Cost::mana(ManaCost::ZERO),
    from: ZoneKind::Graveyard,
    only_from: true,
    exile_after: false,
    condition: None,
    instant_speed: true,
    enters_attacking: false,
    kind: WayKind::Other,
    turn_up: Cost::NONE,
};

static WAY_SPELL: CardDef = CardDef { name: "Route Way Probe", ways: &[HAND_WAY, GRAVEYARD_WAY], ..PROBE_SPELL };

static INTRINSIC_GRAVEYARD_SPELL: CardDef =
    CardDef { name: "Route Intrinsic Graveyard Probe", keywords: Keywords::of(&[Keyword::CastFromYourGraveyard]), ..PROBE_SPELL };

static ADVENTURE_FACE: CardDef = CardDef {
    name: "Route Adventure Probe Adventure",
    cost: ManaCost { generic: 1, ..ManaCost::ZERO },
    types: TypeSet::of(&[CardType::Sorcery]),
    ..PROBE_SPELL
};

static ADVENTURE_PROBE: CardDef = CardDef { name: "Route Adventure Probe", adventure: Some(&ADVENTURE_FACE), ..PROBE_SPELL };

fn cast_opponents_graveyard_card(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let opponent = PlayerId(1 - item.controller.0);
    if let Some(&card) = g.state().zone(Zone::graveyard(opponent)).first() {
        g.cast_during_resolution(card, None)?;
    }
    Ok(())
}

static RESOLUTION_PROBE: CardDef = CardDef {
    name: "Route Resolution Probe",
    cost: ManaCost::ZERO,
    spell: Some(SpellDef { targets: &[], effect: cast_opponents_graveyard_card }),
    ..PROBE_SPELL
};

static FLASHBACK_WAY: WayDef = WayDef { text: "Flashback {0}", exile_after: true, ..GRAVEYARD_WAY };

static FLASHBACK_SPELL: CardDef = CardDef { name: "Route Resolution Flashback Probe", ways: &[FLASHBACK_WAY], ..PROBE_SPELL };

fn cast_own_graveyard_card(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(&card) = g.state().zone(Zone::graveyard(item.controller)).first() {
        g.cast_during_resolution(card, None)?;
    }
    Ok(())
}

static FLASHBACK_INSTRUCTION: CardDef = CardDef {
    name: "Route Flashback Instruction Probe",
    cost: ManaCost::ZERO,
    spell: Some(SpellDef { targets: &[], effect: cast_own_graveyard_card }),
    ..PROBE_SPELL
};

fn play_own_exiled_land(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(&card) = g.state().zone(Zone::exile(item.controller)).first() {
        g.play_land_during_resolution(card)?;
    }
    Ok(())
}

static PLAY_LAND_INSTRUCTION: CardDef = CardDef {
    name: "Route Play Land Instruction Probe",
    cost: ManaCost::ZERO,
    spell: Some(SpellDef { targets: &[], effect: play_own_exiled_land }),
    ..PROBE_SPELL
};

static SPLIT_LEFT: CardDef = CardDef { name: "Route Split Left", ..PROBE_SPELL };
static SPLIT_RIGHT: CardDef = CardDef { name: "Route Split Right", cost: ManaCost { generic: 4, ..ManaCost::ZERO }, ..PROBE_SPELL };
static SPLIT: CardDef = CardDef { name: "Route Split Probe", oracle: true, split: Some((&SPLIT_LEFT, &SPLIT_RIGHT)), ..CardDef::BLANK };

fn any_top_spell(_: &dyn Query, _: &Asker, _: ObjectId) -> bool {
    true
}

static LIFE_TOP: CardDef = CardDef {
    name: "Route Library Life Source",
    types: TypeSet::of(&[CardType::Artifact]),
    play_rules: &[PlayRule::LibraryTop],
    ..CardDef::BLANK
};

static MANA_TOP: CardDef = CardDef {
    name: "Route Library Mana Source",
    types: TypeSet::of(&[CardType::Artifact]),
    play_rules: &[PlayRule::LibraryTopSpells(any_top_spell, None)],
    ..CardDef::BLANK
};

static PERMISSION_SOURCE: CardDef = CardDef { name: "Route Permission Source", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };

fn one_life(_: &dyn Query, _: &Asker, _: ObjectId, _: PlayerId) -> Option<Cost> {
    Some(Cost { life: 1, ..Cost::NONE })
}

fn nothing(_: &dyn Query, _: &Asker, _: ObjectId, _: PlayerId) -> Option<Cost> {
    Some(Cost::NONE)
}

static ONE_LIFE_WAY: GrantedWay = GrantedWay { text: "one life", cost: one_life, ..GrantedWay::CAST };
static FREE_FLASHBACK_WAY: GrantedWay = GrantedWay { text: "free flashback", cost: nothing, flashback: true, ..GrantedWay::CAST };

macro_rules! grantor {
    ($name:literal, $way:ident) => {
        CardDef { name: $name, types: TypeSet::of(&[CardType::Artifact]), granted_ways: &[$way], ..CardDef::BLANK }
    };
}

static GRANTOR_0: CardDef = grantor!("Route Grantor 0", ONE_LIFE_WAY);
static GRANTOR_1: CardDef = grantor!("Route Grantor 1", ONE_LIFE_WAY);
static GRANTOR_2: CardDef = grantor!("Route Grantor 2", ONE_LIFE_WAY);
static GRANTOR_3: CardDef = grantor!("Route Grantor 3", ONE_LIFE_WAY);
static GRANTOR_4: CardDef = grantor!("Route Grantor 4", ONE_LIFE_WAY);
static GRANTOR_5: CardDef = grantor!("Route Grantor 5", ONE_LIFE_WAY);
static GRANTOR_6: CardDef = grantor!("Route Grantor 6", ONE_LIFE_WAY);
static GRANTOR_7: CardDef = grantor!("Route Grantor 7", ONE_LIFE_WAY);
static GRANTOR_8: CardDef = grantor!("Route Grantor 8", FREE_FLASHBACK_WAY);

fn any_card(_: &dyn Query, _: &Asker, _: ObjectId, _: PlayerId) -> bool {
    true
}

fn spells_you_cast(_: &dyn Query, asker: &Asker, _: ObjectId, player: PlayerId) -> bool {
    player == asker.controller
}

static EXILE_PERMISSION: CardDef = CardDef {
    name: "Route Exile Permission",
    types: TypeSet::of(&[CardType::Artifact]),
    exile_casts: &[ExileCast { text: "Cast from exile", allows: any_card, additional: Cost::NONE, any_type: false, lands: false, life_for_mana_value: false }],
    ..CardDef::BLANK
};

/// "You may cast this card from exile" (CR 113.6e): its own static ability, for itself alone.
fn this_card_only(_: &dyn Query, asker: &Asker, card: ObjectId, _: PlayerId) -> bool {
    card == asker.source
}

static SELF_EXILE_PROBE: CardDef = CardDef {
    name: "Route Self Exile Probe",
    cost: ManaCost { generic: 1, ..ManaCost::ZERO },
    exile_casts: &[ExileCast {
        text: "Cast from exile",
        allows: this_card_only,
        additional: Cost::NONE,
        any_type: false,
        lands: false,
        life_for_mana_value: false,
    }],
    ..PROBE_SPELL
};

static FREE_ALTERNATIVE: CardDef = CardDef {
    name: "Route Free Alternative",
    types: TypeSet::of(&[CardType::Artifact]),
    spell_alternatives: &[SpellAlternative {
        text: "Pay {0} rather than its mana cost",
        applies: spells_you_cast,
        cost: InsteadOfManaCost::Mana(ManaCost::ZERO),
        flash: false,
    }],
    ..CardDef::BLANK
};

/// An emblem (CR 114) that gives the spells its owner casts an alternative cost, as Tamiyo, Field Researcher's does.
static FREE_EMBLEM: CardDef = CardDef { name: "Route Free Emblem", types: TypeSet::NONE, ..FREE_ALTERNATIVE };

/// An emblem that gives cards in its owner's graveyard a way to be cast.
static GRANTOR_EMBLEM: CardDef = CardDef { name: "Route Grantor Emblem", types: TypeSet::NONE, ..GRANTOR_0 };

fn exile_it(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.exile(item.object);
    Ok(())
}

/// A suspend-like special action (CR 702.62a, 116.2): taken from the hand whenever the card could be cast.
static SUSPEND_ABILITY: [AbilityDef; 1] =
    [cantrip_api::ability(Cost::mana(ManaCost::ZERO), exile_it, "Suspend probe").timing(Timing::AsCast).from(ZoneKind::Hand).special_action()];

static SUSPEND_PROBE: CardDef = CardDef { name: "Route Suspend Probe", abilities: &SUSPEND_ABILITY, ..PROBE_SPELL };

fn never(_: &dyn Query, _: &Asker) -> bool {
    false
}

/// One that may be cast only when it can't ("cast this spell only ...", CR 601.3).
static SUSPEND_NEVER: CardDef = CardDef { name: "Route Suspend Never Probe", cast_only_if: Some(never), ..SUSPEND_PROBE };

fn no_suspend_probe(q: &dyn Query, _: &Asker, _: PlayerId, spell: ObjectId) -> bool {
    q.card(q.state().obj(spell).card).name != "Route Suspend Probe"
}

/// "Spells named Route Suspend Probe can't be cast" (CR 101.2, `CardDef::spell_rules`: Meddling Mage's kind).
static SUSPEND_FORBIDDER: CardDef =
    CardDef { name: "Route Suspend Forbidder", types: TypeSet::of(&[CardType::Artifact]), spell_rules: &[no_suspend_probe], ..CardDef::BLANK };

/// A spell with split second (CR 702.61a), for nothing.
static SPLIT_SECOND_PROBE: CardDef =
    CardDef { name: "Route Split Second Probe", cost: ManaCost::ZERO, keywords: Keywords::of(&[Keyword::SplitSecond]), ..PROBE_SPELL };

/// "{T}, Pay {E}: Add {C}" (Aether Hub's kind): a mana ability that costs energy.
static ENERGY_MANA_ABILITY: [AbilityDef; 1] = [mana_ability(Cost { tap: true, energy: 1, ..Cost::NONE }, add(Mana::C), "{T}, Pay {E}: Add {C}.")];

static ENERGY_SOURCE: CardDef =
    CardDef { name: "Route Energy Source", types: TypeSet::of(&[CardType::Artifact]), abilities: &ENERGY_MANA_ABILITY, ..CardDef::BLANK };

/// A {1} instant that also costs {E} (CR 107.14, 118.3).
static ENERGY_SPELL: CardDef = CardDef {
    name: "Route Energy Spell",
    cost: ManaCost { generic: 1, ..ManaCost::ZERO },
    additional_cost: Some(Cost { energy: 1, ..Cost::NONE }),
    ..PROBE_SPELL
};

fn nothing_happens(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// "{2}: nothing", an activated ability that isn't a mana ability.
static TWO_ABILITY: [AbilityDef; 1] = [cantrip_api::ability(Cost::mana(ManaCost { generic: 2, ..ManaCost::ZERO }), nothing_happens, "{2}: Nothing.")];

static ABILITY_PROBE: CardDef = CardDef { name: "Route Ability Probe", types: TypeSet::of(&[CardType::Artifact]), abilities: &TWO_ABILITY, ..CardDef::BLANK };

fn free_for_you(_: &dyn Query, asker: &Asker, activator: PlayerId, _: ObjectId, _: &ActivatedDef) -> Option<ManaCost> {
    (activator == asker.controller).then_some(ManaCost::ZERO)
}

/// "You may pay {0} rather than pay the mana cost of abilities you activate" (Forge Anew's and Kili's kind, CR 118.9).
static ABILITY_FREE: CardDef =
    CardDef { name: "Route Ability Free", types: TypeSet::of(&[CardType::Artifact]), ability_alternatives: &[free_for_you], ..CardDef::BLANK };

fn your_lands(view: &dyn LayerQuery, source: ObjectId, id: ObjectId, chars: &Chars) -> bool {
    chars.is(CardType::Land) && view.state().obj(id).zone.is(ZoneKind::Battlefield) && view.state().obj(id).controller == view.state().obj(source).controller
}

fn grant_first_delayed(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let card = view.state().obj(source).card;
    if let Some(slot) = chars.abilities.iter().position(|a| a.is_none()) {
        chars.abilities[slot] = Some(AbilityRef::delayed(card, 0));
    }
}

/// "Lands you control have '{0}: Nothing.'" (Chromatic Lantern's kind, CR 613.1f): the land is the ability's source, and the
/// permanent that gives it is what gave it (CR 113.2c: two of them, two abilities).
static GIVER: CardDef = CardDef {
    name: "Route Ability Giver",
    types: TypeSet::of(&[CardType::Artifact]),
    delayed: &[cantrip_api::ability(Cost::mana(ManaCost::ZERO), nothing_happens, "{0}: Nothing.")],
    statics: &[static_ability(Layer::Ability, your_lands, grant_first_delayed, "Lands you control have \"{0}: Nothing.\"")],
    ..CardDef::BLANK
};

fn gain_energy(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.change_energy(item.controller, 1);
    Ok(())
}

/// "You get {E}" for nothing, so a test has energy the way a game gives it (CR 107.14).
static ENERGY_GAIN: CardDef = CardDef {
    name: "Route Energy Gain",
    cost: ManaCost::ZERO,
    spell: Some(SpellDef { targets: &[], effect: gain_energy }),
    ..PROBE_SPELL
};

fn creature_tapped_for_mana(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match *event {
        Event::TappedForMana { permanent, player, .. } if player == q.controller(source) && q.chars(permanent).is(CardType::Creature) => {
            Fired::about(Target::Object(permanent))
        }
        _ => None,
    }
}

fn add_green(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::G, 1);
    Ok(())
}

/// "Whenever you tap a creature for mana, add an additional {G}" (Badgermole Cub's kind, CR 605.1b).
static CREATURE_TAP_EXTRA: CardDef = CardDef {
    name: "Route Creature Tap Extra",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[triggered(when(EventMask::TAPPED, creature_tapped_for_mana), add_green, "Whenever you tap a creature for mana, add an additional {G}.")
        .mana_ability(TriggeredMana::One(Mana::G))],
    ..CardDef::BLANK
};

/// A creature whose mana ability doesn't tap it ("Pay 1 life: Add {C}"): no tap for mana (CR 106.12).
static UNTAPPED_MANA_CREATURE: CardDef = CardDef {
    name: "Route Untapped Mana Creature",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[mana_ability(Cost { life: 1, ..Cost::NONE }, add(Mana::C), "Pay 1 life: Add {C}.")],
    ..CardDef::BLANK
};

/// One that taps: "{T}: Add {C}".
static TAPPED_MANA_CREATURE: CardDef = CardDef {
    name: "Route Tapped Mana Creature",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")],
    ..CardDef::BLANK
};

/// A {G} instant.
static GREEN_SPELL: CardDef = CardDef { name: "Route Green Spell", cost: ManaCost { symbols: [0, 0, 0, 0, 1, 0], ..ManaCost::ZERO }, ..PROBE_SPELL };

/// "Lands you control have 'Sacrifice [this]: Nothing.'" (Deconstruction Hammer's kind, CR 201.5a): each instance sacrifices
/// the permanent that gave it.
static SACRIFICING_GIVER: CardDef = CardDef {
    name: "Route Sacrificing Giver",
    types: TypeSet::of(&[CardType::Artifact]),
    delayed: &[cantrip_api::ability(Cost { sacrifice_granter: true, ..Cost::NONE }, nothing_happens, "Sacrifice Route Sacrificing Giver: Nothing.")],
    statics: &[static_ability(Layer::Ability, your_lands, grant_first_delayed, "Lands you control have \"Sacrifice Route Sacrificing Giver: Nothing.\"")],
    ..CardDef::BLANK
};

fn your_artifact(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Artifact) && q.controller(id) == asker.controller
}

/// "As an additional cost to cast this spell, you may sacrifice any number of artifacts. This spell costs {1} less to cast
/// for each permanent sacrificed this way" (Rottenmouth Viper's kind, CR 601.2b, 601.2f), {5}.
static SACRIFICE_ANY_NUMBER: CardDef = CardDef {
    name: "Route Sacrifice Any Number",
    cost: ManaCost { generic: 5, ..ManaCost::ZERO },
    additional_cost: Some(Cost { variable: Some(VariableCost::SacrificeAnyNumber { filter: your_artifact, less_each: 1 }), ..Cost::NONE }),
    spell: Some(SpellDef { targets: &[], effect: nothing_happens }),
    ..PROBE_SPELL
};

static ROUTE_ARTIFACT: CardDef = CardDef { name: "Route Artifact", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };

fn choose_one_card_of_hand(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let hand = g.state().zone(Zone::hand(item.controller)).to_vec();
    let ask = RequestKind::choose_cards(item.object, Purpose::Discard, hand.clone(), hand, 1, false, "");
    let _ = g.ask(item.controller, ask)?;
    Ok(())
}

/// "Choose a card in your hand", for nothing: a question of one card among many.
static CHOOSE_ONE: CardDef = CardDef { name: "Route Choose One", cost: ManaCost::ZERO, spell: Some(SpellDef { targets: &[], effect: choose_one_card_of_hand }), ..PROBE_SPELL };

static FILLER: CardDef = CardDef { name: "Route Filler", has_cost: false, ..CardDef::BLANK };

static LAND: CardDef = CardDef { name: "Route Land", oracle: true, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        CardTable::new([
            &PROBE_SPELL,
            &WAY_SPELL,
            &INTRINSIC_GRAVEYARD_SPELL,
            &ADVENTURE_PROBE,
            &ADVENTURE_FACE,
            &RESOLUTION_PROBE,
            &FLASHBACK_SPELL,
            &FLASHBACK_INSTRUCTION,
            &PLAY_LAND_INSTRUCTION,
            &SPLIT_LEFT,
            &SPLIT_RIGHT,
            &SPLIT,
            &LIFE_TOP,
            &MANA_TOP,
            &PERMISSION_SOURCE,
            &FILLER,
            &GRANTOR_0,
            &GRANTOR_1,
            &GRANTOR_2,
            &GRANTOR_3,
            &GRANTOR_4,
            &GRANTOR_5,
            &GRANTOR_6,
            &GRANTOR_7,
            &GRANTOR_8,
            &EXILE_PERMISSION,
            &SELF_EXILE_PROBE,
            &FREE_ALTERNATIVE,
            &FREE_EMBLEM,
            &SUSPEND_PROBE,
            &ABILITY_PROBE,
            &ABILITY_FREE,
            &GIVER,
            &SACRIFICE_ANY_NUMBER,
            &ROUTE_ARTIFACT,
            &CHOOSE_ONE,
            &SACRIFICING_GIVER,
            &ENERGY_SOURCE,
            &ENERGY_SPELL,
            &ENERGY_GAIN,
            &CREATURE_TAP_EXTRA,
            &UNTAPPED_MANA_CREATURE,
            &TAPPED_MANA_CREATURE,
            &GREEN_SPELL,
            &SUSPEND_NEVER,
            &SUSPEND_FORBIDDER,
            &SPLIT_SECOND_PROBE,
            &GRANTOR_EMBLEM,
            &LAND,
        ])
    })
}

/// A game in the first player's precombat main phase with priority, their first deck card where `to` says, `colorless`
/// mana in their pool, and the rest of both decks filler.
fn game_with(name: &str, to: fn(PlayerId) -> Zone, colorless: u8) -> (RulesGame, ObjectId) {
    let cards = table();
    let card = cards.find(name).unwrap();
    let filler = cards.find("Route Filler").unwrap();
    let mut game = RulesGame::new(cards, &Setup { decks: vec![vec![card, filler, filler, filler, filler, filler, filler], vec![filler; 7]], seed: 1, ..Setup::default() });
    let id = game.state.zone(Zone::hand(ME))[0];
    move_to(&mut game, id, to(ME));
    begin(&mut game, colorless);
    (game, id)
}

fn move_to(game: &mut RulesGame, id: ObjectId, zone: Zone) {
    let from = game.state.obj(id).zone;
    assert!(game.state.zone_remove(from, id));
    game.state.update_obj(id, |o| o.zone = zone);
    game.state.zone_push(zone, id);
}

fn begin(game: &mut RulesGame, colorless: u8) {
    game.state.update_turn(|t| {
        t.number = 1;
        t.active = ME;
        t.priority = ME;
        t.set_step(Step::PrecombatMain);
        t.began = true;
    });
    game.state.update_player(ME, |p| p.pool.0[Mana::C as usize] = u32::from(colorless));
    game.state.frame_push(Frame::new(FrameKind::Priority));
}

fn put(game: &mut RulesGame, name: &str, player: PlayerId) -> ObjectId {
    let card = game.cards.find(name).unwrap();
    let id = game.state.push_obj(Obj::new(card, player, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

fn permission(card: ObjectId, by: ObjectId, grant: PlayGrant) -> PlayPermission {
    PlayPermission { what: Playable::Card(card), player: ME, grant, by: Grant { object: by, ability: AbilityRef::spell(CardId(0)) }, turn: 1 }
}

fn casts_of(game: &RulesGame, card: ObjectId) -> Vec<ActionOption> {
    match &game.pending().expect("a priority request").kind {
        RequestKind::Priority { options } => options.iter().filter(|o| o.kind == ActionKind::CastSpell && o.source == Some(card)).cloned().collect(),
        other => panic!("expected priority, got {other:?}"),
    }
}

fn by(option: &ActionOption) -> Option<ObjectId> {
    option.granted_by.map(|g| g.object)
}

fn take(game: &mut RulesGame, option: &ActionOption) {
    let request = game.pending().unwrap().clone();
    game.answer(request.id, request.player, Answer::Act(Action::of(option))).unwrap();
}

/// Plays on until the stack is empty again after a cast: passes, takes a printed way or another object's alternative cost
/// where a cost is to be chosen, and confirms a payment.
fn resolve(game: &mut RulesGame) {
    for _ in 0..24 {
        game.run();
        let cast = game.log.iter().any(|r| matches!(r.event, Event::SpellCast { player: ME, .. }));
        if game.state.stack.is_empty() && cast {
            return;
        }
        let request = game.pending().cloned().expect("asked before the cast resolves");
        let answer = match request.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseModes { modes, eligible, .. } => {
                let picked = eligible.iter().copied().find(|&i| matches!(modes[i as usize], Label::Way(..) | Label::AlternativeCost(..)));
                Answer::Modes(vec![picked.or(eligible.first().copied()).expect("a mode")])
            }
            RequestKind::PayCosts { options, .. } => {
                let confirm = options
                    .iter()
                    .find(|o| o.kind == ActionKind::AssignMana)
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::ConfirmCasting))
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::CancelCasting))
                    .unwrap_or_else(|| panic!("unexpected payment options: {options:?}"));
                Answer::Act(Action::of(confirm))
            }
            other => panic!("unexpected request: {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the cast did not resolve");
}

fn in_zone(game: &RulesGame, zone: Zone, card: CardId) -> bool {
    game.state.zone(zone).iter().any(|&id| game.state.obj(id).card == card)
}

/// CR 608.2g, 702.34a: a card an effect has cast from its owner's graveyard as it resolves may be cast for its flashback
/// cost, and is exiled as it leaves the stack.
#[test]
fn a_cast_as_an_effect_resolves_may_use_the_cards_own_flashback_from_its_graveyard() {
    let cards = table();
    let (instruction, flashback, filler) = (cards.find("Route Flashback Instruction Probe").unwrap(), cards.find("Route Resolution Flashback Probe").unwrap(), cards.find("Route Filler").unwrap());
    let mut game = RulesGame::new(cards, &Setup { decks: vec![vec![instruction, flashback, filler, filler, filler, filler, filler], vec![filler; 7]], seed: 1, ..Setup::default() });
    let (instruction_id, flashback_id) = (game.state.zone(Zone::hand(ME))[0], game.state.zone(Zone::hand(ME))[1]);
    move_to(&mut game, flashback_id, Zone::graveyard(ME));
    begin(&mut game, 0);
    game.run();
    let option = casts_of(&game, instruction_id).into_iter().next().expect("the instruction is castable");
    take(&mut game, &option);
    resolve(&mut game);
    let cast = game.log.iter().find_map(|r| match r.event {
        Event::Cast(c) if c.from == ZoneKind::Graveyard => Some(c),
        _ => None,
    });
    assert_eq!(cast.expect("the graveyard card was cast").way, 1, "by its printed flashback");
    assert!(in_zone(&game, Zone::exile(ME), flashback) && !in_zone(&game, Zone::graveyard(ME), flashback));
}

/// CR 601.3, 118.9, 709.3a: a split card on top of the library is offered by each permission for each half - for life
/// equal to that half's mana value by one, for that half's mana cost by the other - and the half chosen is cast.
#[test]
fn each_permission_for_the_top_of_the_library_offers_each_split_half_at_its_own_cost() {
    let (mut game, top) = game_with("Route Split Probe", Zone::library, 4);
    let library = game.state.zone(Zone::library(ME)).to_vec();
    game.state.zone_remove(Zone::library(ME), top);
    game.state.zone_insert(Zone::library(ME), 0, top);
    assert_eq!(game.state.zone(Zone::library(ME)).len(), library.len());
    let life = put(&mut game, "Route Library Life Source", ME);
    let mana = put(&mut game, "Route Library Mana Source", ME);
    game.run();
    let options = casts_of(&game, top);
    let of = |source| options.iter().filter(|o| by(o) == Some(source)).collect::<Vec<_>>();
    let mut lives: Vec<u8> = of(life).iter().map(|o| cantrip_rules::life_for_cast(&game, game.cards, ME, o).expect("for life")).collect();
    lives.sort_unstable();
    assert_eq!(lives, [2, 4], "life equal to each half's own mana value");
    let by_mana = of(mana);
    assert_eq!(by_mana.len(), 2, "each half for mana");
    assert!(by_mana.iter().all(|o| cantrip_rules::life_for_cast(&game, game.cards, ME, o).is_none()));
    let four = (*by_mana.iter().find(|o| o.index == WAY_SPLIT_RIGHT).expect("the four-mana half")).clone();
    take(&mut game, &four);
    resolve(&mut game);
    assert!(game.log.iter().any(|r| matches!(r.event, Event::Cast(c) if c.from == ZoneKind::Library && c.value(&game) == 4 && c.granted_by().is_some_and(|g| g.object == mana))));
    assert_eq!(game.state.player(ME).life, 20);
}

/// CR 118.9a: a permission that names its own alternative cost (life for the mana value) opens no other; one that casts for
/// the costs the card has opens its alternative cost that names no zone.
#[test]
fn a_printed_alternative_cost_is_open_by_the_permission_that_names_none_only() {
    let (mut game, top) = game_with("Route Way Probe", Zone::library, 0);
    game.state.zone_remove(Zone::library(ME), top);
    game.state.zone_insert(Zone::library(ME), 0, top);
    let life = put(&mut game, "Route Library Life Source", ME);
    let mana = put(&mut game, "Route Library Mana Source", ME);
    game.run();
    let options = casts_of(&game, top);
    assert!(!options.iter().any(|o| by(o) == Some(life) && o.index == 1));
    assert!(options.iter().any(|o| by(o) == Some(mana) && o.index == 1));
    // The way naming the graveyard is no way from the library.
    assert!(!options.iter().any(|o| o.index == 2));
}

/// CR 601.2b, 601.3: two permissions for one card are two casts, whichever was given first, and the one chosen is the one
/// the cast is made by: for nothing, not for life, and with no flashback.
#[test]
fn two_play_permissions_are_two_casts_and_the_chosen_one_is_made() {
    for free_first in [true, false] {
        let (mut game, spell) = game_with("Route Probe", Zone::graveyard, 0);
        let (life_source, free_source) = (put(&mut game, "Route Permission Source", ME), put(&mut game, "Route Permission Source", ME));
        let life = permission(spell, life_source, PlayGrant { life_for_mana_value: true, flashback: true, ..PlayGrant::CAST });
        let free = permission(spell, free_source, PlayGrant { instead: Some(ManaCost::ZERO), ..PlayGrant::CAST });
        for p in if free_first { [free, life] } else { [life, free] } {
            game.state.permission_push(p);
        }
        game.run();
        let options = casts_of(&game, spell);
        let free = options.iter().find(|o| by(o) == Some(free_source)).expect("the free permission's cast").clone();
        let life = options.iter().find(|o| by(o) == Some(life_source)).expect("the life permission's cast");
        assert_eq!(cantrip_rules::life_for_cast(&game, game.cards, ME, life), Some(2));
        assert_eq!(cantrip_rules::life_for_cast(&game, game.cards, ME, &free), None);
        take(&mut game, &free);
        resolve(&mut game);
        assert_eq!(game.state.player(ME).life, 20, "no life paid");
        assert!(in_zone(&game, Zone::graveyard(ME), table().find("Route Probe").unwrap()), "no flashback: back to the graveyard");
    }
}

/// CR 601.3: the card's own "you may cast this card from your graveyard", the effect of Yawgmoth's Will and the permission
/// a card exiled on an Adventure has each stay a cast of its own beside a permission that names a cost, and are made as
/// themselves: recorded so, and back to the graveyard rather than into the exile the other's flashback would send it to.
#[test]
fn a_rule_or_the_cards_own_permission_stays_beside_a_permission_that_names_a_cost() {
    let forced = PlayGrant { instead: Some(ManaCost::ZERO), flashback: true, ..PlayGrant::CAST };
    for case in 0..3 {
        let (name, zone): (&str, fn(PlayerId) -> Zone) = match case {
            0 => ("Route Intrinsic Graveyard Probe", Zone::graveyard),
            1 => ("Route Probe", Zone::graveyard),
            _ => ("Route Adventure Probe", Zone::exile),
        };
        let (mut game, spell) = game_with(name, zone, 2);
        let source = put(&mut game, "Route Permission Source", ME);
        if case == 1 {
            game.state.effect_push(ContinuousEffect {
                source,
                controller: ME,
                timestamp: 1,
                affected: source,
                kind: EffectKind::PlayFromGraveyard,
                duration: Duration::Indefinite,
            });
        }
        if case == 2 {
            game.state.update_obj(spell, |o| o.on_adventure = Some(ME));
        }
        game.state.permission_push(permission(spell, source, forced));
        game.run();
        let options = casts_of(&game, spell);
        let forced_cast = options.iter().find(|o| o.index == 0 && game.state.permissions.iter().any(|p| Some(p.by) == o.granted_by));
        assert!(forced_cast.is_some(), "case {case}: the permission's own cast");
        let ordinary = options
            .iter()
            .find(|o| o.index == 0 && !game.state.permissions.iter().any(|p| Some(p.by) == o.granted_by))
            .unwrap_or_else(|| panic!("case {case}: the other permission's cast: {options:?}"))
            .clone();
        take(&mut game, &ordinary);
        resolve(&mut game);
        let card = game.state.obj(spell).card;
        assert!(in_zone(&game, Zone::graveyard(ME), card) && !in_zone(&game, Zone::exile(ME), card), "case {case}");
        assert!(game.log.iter().any(|r| matches!(r.event, Event::Cast(c) if c.granted_by() == ordinary.granted_by && c.way == 0)), "case {case}");
    }
}

/// An action naming a permission the question did not offer it by is refused, and the question stays.
#[test]
fn a_cast_by_a_permission_not_offered_is_refused() {
    let (mut game, spell) = game_with("Route Probe", Zone::graveyard, 0);
    let source = put(&mut game, "Route Permission Source", ME);
    game.state.permission_push(permission(spell, source, PlayGrant { instead: Some(ManaCost::ZERO), ..PlayGrant::CAST }));
    game.run();
    let option = casts_of(&game, spell).into_iter().next().unwrap();
    let request = game.pending().unwrap().clone();
    let mut forged = Action::of(&option);
    forged.granted_by = Some(Grant { object: source, ability: AbilityRef::spell(CardId(1)) });
    assert!(game.answer(request.id, request.player, Answer::Act(forged.clone())).is_err());
    forged.granted_by = None;
    assert!(game.answer(request.id, request.player, Answer::Act(forged)).is_err());
    assert!(game.pending().is_some());
}

/// An action naming an alternative cost the question did not offer it for is refused too: by a permanent that gives none,
/// or beside a permission that names its own cost (CR 118.9a).
#[test]
fn a_cast_for_an_alternative_cost_not_offered_is_refused() {
    let (mut game, spell) = game_with("Route Probe", Zone::graveyard, 1);
    let source = put(&mut game, "Route Permission Source", ME);
    let payer = put(&mut game, "Route Free Alternative", ME);
    game.state.permission_push(permission(spell, source, PlayGrant { instead: Some(ManaCost { generic: 1, ..ManaCost::ZERO }), ..PlayGrant::CAST }));
    game.run();
    let request = game.pending().unwrap().clone();
    let option = casts_of(&game, spell).into_iter().find(|o| by(o) == Some(source)).expect("the permission's cast");
    assert!(option.paid_by.is_none());
    for paid_by in [
        Grant { object: payer, ability: AbilityRef::spell_alternative(game.state.obj(payer).card, 0) },
        Grant { object: source, ability: AbilityRef::spell_alternative(game.state.obj(source).card, 0) },
    ] {
        let mut forged = Action::of(&option);
        forged.paid_by = Some(paid_by);
        assert!(game.answer(request.id, request.player, Answer::Act(forged)).is_err());
    }
    assert!(game.pending().is_some());
}

/// CR 305.1, 614.1c: a land two permissions would each let be played is two land plays; the one chosen is the one it is
/// played by, so it enters tapped only by the permission that says so, and the history records which it was.
#[test]
fn a_land_two_permissions_let_be_played_is_played_by_the_one_chosen() {
    for tapped in [true, false] {
        let (mut game, land) = game_with("Route Land", Zone::exile, 0);
        let (slow, fast) = (put(&mut game, "Route Permission Source", ME), put(&mut game, "Route Permission Source", ME));
        let lands = |enters_tapped| PlayGrant { lands: true, enters_tapped, ..PlayGrant::CAST };
        game.state.permission_push(permission(land, slow, lands(true)));
        game.state.permission_push(permission(land, fast, lands(false)));
        game.run();
        let request = game.pending().unwrap().clone();
        let RequestKind::Priority { options } = &request.kind else { panic!("priority") };
        let plays: Vec<ActionOption> = options.iter().filter(|o| o.kind == ActionKind::PlayLand && o.source == Some(land)).cloned().collect();
        assert_eq!(plays.len(), 2, "{plays:?}");
        let want = if tapped { slow } else { fast };
        let play = plays.iter().find(|o| by(o) == Some(want)).expect("each permission's play");
        game.answer(request.id, request.player, Answer::Act(Action::of(play))).unwrap();
        game.run();
        let card = table().find("Route Land").unwrap();
        let on = game.state.battlefield().iter().copied().find(|&id| game.state.obj(id).card == card).expect("on the battlefield");
        assert_eq!(game.state.obj(on).tapped, tapped);
        assert!(game.log.iter().any(|r| matches!(r.event, Event::LandPlayed { land, granted_by, .. } if land == on && granted_by.some().map(|g| g.object) == Some(want))));
    }
}

/// CR 305.1, 608.2g: a land a resolving effect plays is played by that effect, not "this way" by a permission that stands
/// on the card, so the permission's "each land played this way enters tapped" has no say (CR 614.1c); and the play is
/// recorded, with nothing beyond the effect named as what made it possible.
#[test]
fn a_land_a_resolving_effect_plays_ignores_a_permissions_enters_tapped_and_is_recorded() {
    let cards = table();
    let (instruction, land_card, filler) =
        (cards.find("Route Play Land Instruction Probe").unwrap(), cards.find("Route Land").unwrap(), cards.find("Route Filler").unwrap());
    let mut game = RulesGame::new(cards, &Setup { decks: vec![vec![instruction, land_card, filler, filler, filler, filler, filler], vec![filler; 7]], seed: 1, ..Setup::default() });
    let (instruction_id, land) = (game.state.zone(Zone::hand(ME))[0], game.state.zone(Zone::hand(ME))[1]);
    move_to(&mut game, land, Zone::exile(ME));
    begin(&mut game, 0);
    let source = put(&mut game, "Route Permission Source", ME);
    game.state.permission_push(permission(land, source, PlayGrant { lands: true, enters_tapped: true, ..PlayGrant::CAST }));
    game.run();
    let option = casts_of(&game, instruction_id).into_iter().next().expect("the instruction is castable");
    take(&mut game, &option);
    resolve(&mut game);
    let on = game.state.battlefield().iter().copied().find(|&id| game.state.obj(id).card == land_card).expect("the land was played");
    assert!(!game.state.obj(on).tapped, "not played by the permission, so not tapped by it");
    assert_eq!(game.state.player(ME).lands_played, 1);
    assert!(game.log.iter().any(|r| matches!(r.event, Event::LandPlayed { land, granted_by, .. } if land == on && granted_by.some().is_none())));
}

/// CR 118.9a: by a permission that names its own cost, the card's zone-free alternative cost is not offered; by one that
/// does not, it is; and the card's own way from its graveyard is a cast of its own either way.
#[test]
fn a_permission_naming_a_cost_borrows_no_alternative_cost() {
    let (mut game, spell) = game_with("Route Way Probe", Zone::graveyard, 0);
    let (imposed, ordinary) = (put(&mut game, "Route Permission Source", ME), put(&mut game, "Route Permission Source", ME));
    game.state.permission_push(permission(spell, imposed, PlayGrant { instead: Some(ManaCost::ZERO), ..PlayGrant::CAST }));
    game.state.permission_push(permission(spell, ordinary, PlayGrant::CAST));
    game.run();
    let options = casts_of(&game, spell);
    assert!(!options.iter().any(|o| by(o) == Some(imposed) && o.index == 1));
    assert!(options.iter().any(|o| by(o) == Some(ordinary) && o.index == 1));
    assert!(options.iter().any(|o| o.granted_by.is_none() && o.index == 2));
}

/// CR 608.2g, 601.3: an effect that has a card in another player's graveyard cast is the permission; a way a permanent
/// gives cards in its controller's own graveyard has nothing to do with it, so the card is cast for its mana cost or not
/// at all (here, with no mana, not).
#[test]
fn a_cast_as_an_effect_resolves_uses_no_way_a_permanent_gives() {
    let cards = table();
    let (resolver, spell, filler) = (cards.find("Route Resolution Probe").unwrap(), cards.find("Route Probe").unwrap(), cards.find("Route Filler").unwrap());
    let mut game = RulesGame::new(cards, &Setup { decks: vec![vec![resolver, filler, filler, filler, filler, filler, filler], vec![spell, filler, filler, filler, filler, filler, filler]], seed: 1, ..Setup::default() });
    let resolver_id = game.state.zone(Zone::hand(ME))[0];
    let target = game.state.zone(Zone::hand(PlayerId(1)))[0];
    move_to(&mut game, target, Zone::graveyard(PlayerId(1)));
    begin(&mut game, 0);
    put(&mut game, "Route Grantor 8", ME);
    game.run();
    let option = casts_of(&game, resolver_id).into_iter().next().unwrap();
    take(&mut game, &option);
    resolve(&mut game);
    assert_eq!(game.state.obj(target).zone, Zone::graveyard(PlayerId(1)));
}

fn activations_of(game: &RulesGame, card: ObjectId) -> Vec<ActionOption> {
    match &game.pending().expect("a priority request").kind {
        RequestKind::Priority { options } => options.iter().filter(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(card)).cloned().collect(),
        other => panic!("expected priority, got {other:?}"),
    }
}

/// CR 702.62a: suspend is taken only when the card could begin to be cast - not when its own "cast this spell only" fails,
/// not when a static ability forbids casting it (the rulings: "any other effects that could stop you from casting it"), and
/// not while a spell with split second is on the stack (CR 702.61a), though split second lets special actions be taken.
#[test]
fn suspend_is_taken_only_when_the_card_could_be_cast() {
    let (mut game, card) = game_with("Route Suspend Probe", Zone::hand, 0);
    game.run();
    assert_eq!(activations_of(&game, card).len(), 1, "with nothing in the way, it is suspended");

    let (mut game, card) = game_with("Route Suspend Never Probe", Zone::hand, 0);
    game.run();
    assert!(activations_of(&game, card).is_empty(), "its own condition is not met");

    let (mut game, card) = game_with("Route Suspend Probe", Zone::hand, 0);
    put(&mut game, "Route Suspend Forbidder", PlayerId(1));
    game.run();
    assert!(activations_of(&game, card).is_empty(), "a ban on casting it");

    let cards = table();
    let (probe, split, filler) = (cards.find("Route Suspend Probe").unwrap(), cards.find("Route Split Second Probe").unwrap(), cards.find("Route Filler").unwrap());
    let mut game = RulesGame::new(cards, &Setup { decks: vec![vec![probe, split, filler, filler, filler, filler, filler], vec![filler; 7]], seed: 1, ..Setup::default() });
    let (card, split_id) = (game.state.zone(Zone::hand(ME))[0], game.state.zone(Zone::hand(ME))[1]);
    begin(&mut game, 0);
    game.run();
    let cast = casts_of(&game, split_id).into_iter().next().expect("the split-second spell is castable");
    take(&mut game, &cast);
    for _ in 0..8 {
        game.run();
        let request = game.pending().cloned().expect("asked");
        match request.kind {
            RequestKind::Priority { .. } => break,
            RequestKind::PayCosts { options, .. } => {
                let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("confirm");
                game.answer(request.id, request.player, Answer::Act(Action::of(confirm))).unwrap();
            }
            other => panic!("unexpected request: {other:?}"),
        }
    }
    assert!(game.state.stack.iter().any(|i| i.object != card), "the split-second spell is on the stack");
    assert!(activations_of(&game, card).is_empty(), "split second: the card could not be cast");
}

/// CR 601.2g-h, 107.14: an energy cost is paid with the rest of the cost, after the mana abilities activated to pay it.
/// One that spent the energy the spell needs leaves a cost that can't be paid, and the cast is taken back; with energy to
/// spare it is cast.
#[test]
fn an_energy_cost_is_judged_again_after_the_mana_abilities() {
    let cards = table();
    let (spell_card, gain, filler) = (cards.find("Route Energy Spell").unwrap(), cards.find("Route Energy Gain").unwrap(), cards.find("Route Filler").unwrap());
    for energy in [1usize, 2] {
        let mut game = RulesGame::new(cards, &Setup { decks: vec![vec![spell_card, gain, gain, filler, filler, filler, filler], vec![filler; 7]], seed: 1, ..Setup::default() });
        let spell = game.state.zone(Zone::hand(ME))[0];
        let gains: Vec<ObjectId> = game.state.zone(Zone::hand(ME))[1..1 + energy].to_vec();
        begin(&mut game, 0);
        let source = put(&mut game, "Route Energy Source", ME);
        for g in gains {
            game.run();
            let cast = casts_of(&game, g).into_iter().next().expect("the energy for nothing");
            take(&mut game, &cast);
            resolve(&mut game);
        }
        assert_eq!(game.state.player(ME).energy() as usize, energy);
        game.run();
        let cast = casts_of(&game, spell).into_iter().next().expect("offered: the energy and the mana are each there");
        take(&mut game, &cast);
        for _ in 0..8 {
            game.run();
            let request = game.pending().cloned().expect("asked");
            let RequestKind::PayCosts { options, .. } = &request.kind else { break };
            let pick = options
                .iter()
                .find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(source))
                .or_else(|| options.iter().find(|o| o.kind == ActionKind::ConfirmCasting))
                .or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana))
                .unwrap_or_else(|| panic!("unexpected payment options: {options:?}"));
            game.answer(request.id, request.player, Answer::Act(Action::of(pick))).unwrap();
        }
        let cast_made = game.state.stack.iter().any(|i| game.state.obj(i.object).card == spell_card);
        if energy == 1 {
            assert!(!cast_made, "the mana ability spent the energy the spell needs");
            assert!(game.state.zone(Zone::hand(ME)).contains(&spell), "taken back to the hand");
            assert_eq!(game.state.player(ME).energy(), 1, "and the energy with it");
        } else {
            assert!(cast_made, "with energy to spare it is cast");
            assert_eq!(game.state.player(ME).energy(), 0);
        }
    }
}

/// CR 118.9, 602.2b: each cost a permanent lets a player pay rather than an ability's mana cost is an activation of its own,
/// made by that permanent (`paid_by`), beside the ability for its own cost where that can be paid; the one chosen is paid.
#[test]
fn each_alternative_cost_of_an_ability_is_an_activation_of_its_own() {
    for pool in [0u8, 2] {
        let (mut game, _) = game_with("Route Filler", Zone::hand, pool);
        let ability = put(&mut game, "Route Ability Probe", ME);
        let (first, second) = (put(&mut game, "Route Ability Free", ME), put(&mut game, "Route Ability Free", ME));
        put(&mut game, "Route Ability Free", PlayerId(1));
        game.run();
        let options = activations_of(&game, ability);
        let payers: Vec<Option<ObjectId>> = options.iter().map(|o| o.paid_by.map(|g| g.object)).collect();
        let own = if pool == 0 { vec![] } else { vec![None] };
        assert_eq!(payers, [own, vec![Some(first), Some(second)]].concat(), "the opponent's gives nothing to this player");
        let texts: Vec<String> = options.iter().map(|o| describe(&game, o)).collect();
        assert!(texts.iter().filter(|t| t.contains("paying {0} rather than its mana cost (Route Ability Free)")).count() == 2, "{texts:?}");
        let chosen = options.iter().find(|o| o.paid_by.is_some_and(|g| g.object == second)).unwrap().clone();
        let request = game.pending().unwrap().clone();
        game.answer(request.id, request.player, Answer::Act(Action::of(&chosen))).unwrap();
        game.run();
        assert!(game.state.stack.iter().any(|i| i.object == ability), "activated without being asked anything more");
        assert_eq!(game.state.player(ME).pool.0[Mana::C as usize], u32::from(pool), "for nothing");
    }
}

/// CR 113.2c, 613.1f: an ability another permanent gives is offered and recorded with that permanent (`granted_by`, the
/// history's `AbilityActivated::by`): two givers of one ability are two abilities, each its own. The land's own abilities
/// name nothing.
#[test]
fn an_ability_another_permanent_gives_is_activated_by_it() {
    let (mut game, _) = game_with("Route Filler", Zone::hand, 0);
    let land = put(&mut game, "Route Land", ME);
    let (first, second) = (put(&mut game, "Route Ability Giver", ME), put(&mut game, "Route Ability Giver", ME));
    game.state.see_statics();
    game.run();
    let options = activations_of(&game, land);
    let by: Vec<Option<ObjectId>> = options.iter().map(|o| o.granted_by.map(|g| g.object)).collect();
    assert_eq!(by, [Some(first), Some(second)], "{options:?}");
    let chosen = options[1].clone();
    let request = game.pending().unwrap().clone();
    game.answer(request.id, request.player, Answer::Act(Action::of(&chosen))).unwrap();
    game.run();
    let recorded: Vec<Option<ObjectId>> = game
        .log
        .iter()
        .filter_map(|r| match r.event {
            Event::AbilityActivated { source, by, .. } if source == land => Some(by.some().map(|g| g.object)),
            _ => None,
        })
        .collect();
    assert_eq!(recorded, [Some(second)]);
}

/// CR 201.5a, 113.2c: "Sacrifice [the card that gives this ability]" in an ability two permanents give is each instance's own
/// giver. Activating the second instance sacrifices the second giver, the first stays, and the history records the
/// sacrificed one as what gave it, read before the cost moved it.
#[test]
fn an_ability_that_sacrifices_what_gave_it_sacrifices_its_own_giver() {
    let (mut game, _) = game_with("Route Filler", Zone::hand, 0);
    let land = put(&mut game, "Route Land", ME);
    let (first, second) = (put(&mut game, "Route Sacrificing Giver", ME), put(&mut game, "Route Sacrificing Giver", ME));
    game.state.see_statics();
    game.run();
    let options = activations_of(&game, land);
    assert_eq!(options.iter().map(|o| o.granted_by.map(|g| g.object)).collect::<Vec<_>>(), [Some(first), Some(second)]);
    let chosen = options[1].clone();
    let request = game.pending().unwrap().clone();
    game.answer(request.id, request.player, Answer::Act(Action::of(&chosen))).unwrap();
    game.run();
    assert!(game.state.obj(first).zone.is(ZoneKind::Battlefield), "the other giver stays");
    assert!(!game.state.battlefield().contains(&second), "its own giver is sacrificed");
    let recorded: Vec<Option<ObjectId>> = game
        .log
        .iter()
        .filter_map(|r| match r.event {
            Event::AbilityActivated { source, by, .. } if source == land => Some(by.some().map(|g| g.object)),
            _ => None,
        })
        .collect();
    assert_eq!(recorded, [Some(second)]);
}

/// CR 601.2b, 601.2f: "sacrifice any number of" is any number, past 255: 300 artifacts announced and sacrificed take {300}
/// off a {5} spell, and every one is sacrificed.
#[test]
fn any_number_sacrificed_is_any_number_past_255() {
    let (mut game, spell) = game_with("Route Sacrifice Any Number", Zone::hand, 0);
    let artifacts: Vec<ObjectId> = (0..300).map(|_| put(&mut game, "Route Artifact", ME)).collect();
    game.run();
    let cast = casts_of(&game, spell).into_iter().next().expect("offered: 300 artifacts take all of {5} off");
    take(&mut game, &cast);
    let mut chose = 0;
    for _ in 0..2000 {
        game.run();
        let Some(request) = game.pending().cloned() else { break };
        let answer = match &request.kind {
            RequestKind::PayCosts { announcing: true, options, x, .. } => {
                let kind = if *x < 300 { ActionKind::IncreaseX } else { ActionKind::ConfirmCasting };
                Answer::Act(Action::of(options.iter().find(|o| o.kind == kind).unwrap_or_else(|| panic!("{kind:?} at {x}"))))
            }
            RequestKind::ChooseCards { rule, .. } => {
                chose = rule.count.most().unwrap_or(0);
                Answer::Cards(cantrip_rules::first_choices(&game, &request))
            }
            RequestKind::PayCosts { options, .. } => Answer::Act(Action::of(options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("nothing left to pay"))),
            RequestKind::Priority { .. } => break,
            other => panic!("unexpected request: {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert_eq!(chose, 300);
    assert!(artifacts.iter().all(|&a| !game.state.battlefield().contains(&a)), "every one sacrificed");
    assert!(game.state.stack.iter().any(|i| i.object != spell), "cast, for nothing");
}

/// The core refuses an answer of the wrong number of cards however many it is: 257 cards answering "choose one" is 257,
/// never 1 (as a count narrowed to a byte would read it).
#[test]
fn an_answer_of_257_cards_to_choose_one_is_refused() {
    let cards = table();
    let (choose, filler) = (cards.find("Route Choose One").unwrap(), cards.find("Route Filler").unwrap());
    let mut deck = vec![choose];
    deck.extend(std::iter::repeat_n(filler, 299));
    let mut game = RulesGame::new(cards, &Setup { decks: vec![deck, vec![filler; 7]], seed: 1, ..Setup::default() });
    let spell = game.state.zone(Zone::hand(ME))[0];
    for id in game.state.zone(Zone::library(ME)).to_vec() {
        move_to(&mut game, id, Zone::hand(ME));
    }
    begin(&mut game, 0);
    game.run();
    let cast = casts_of(&game, spell).into_iter().next().expect("castable");
    take(&mut game, &cast);
    for _ in 0..8 {
        game.run();
        let request = game.pending().cloned().expect("asked");
        match &request.kind {
            RequestKind::ChooseCards { rule, .. } if rule.count == Count::Exactly(1) => {
                let eligible = cantrip_rules::first_choices(&game, &request);
                assert!(eligible.len() >= 257);
                let rejected = game.answer(request.id, request.player, Answer::Cards(eligible[..257].to_vec()));
                assert!(rejected.is_err(), "257 cards are not one");
                game.answer(request.id, request.player, Answer::Cards(vec![eligible[0]])).unwrap();
                return;
            }
            RequestKind::PayCosts { options, .. } => {
                let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).unwrap();
                game.answer(request.id, request.player, Answer::Act(Action::of(confirm))).unwrap();
            }
            _ => game.answer(request.id, request.player, Answer::Pass).unwrap(),
        }
    }
    panic!("never asked to choose");
}

fn describe(game: &RulesGame, option: &ActionOption) -> String {
    cantrip_rules::describe(game.cards, &game.state, &game.log, option).to_string()
}

/// CR 106.12, 605.1b: a triggered mana ability that waits on a tap counts towards what can be paid only for a mana ability
/// that taps its permanent. "Whenever you tap a creature for mana, add an additional {G}" makes a {G} spell castable from a
/// creature's "{T}: Add {C}" (the {G} pays it), and not from its "Pay 1 life: Add {C}".
#[test]
fn a_tap_trigger_counts_only_for_a_mana_ability_that_taps() {
    for (creature, offered) in [("Route Untapped Mana Creature", false), ("Route Tapped Mana Creature", true)] {
        let (mut game, spell) = game_with("Route Green Spell", Zone::hand, 0);
        let id = put(&mut game, creature, ME);
        // Under its controller's control since before their turn began: no summoning sickness (CR 302.6).
        game.state.update_player(ME, |p| p.turn_began = 1);
        game.state.update_obj(id, |o| o.controlled_since = 0);
        put(&mut game, "Route Creature Tap Extra", ME);
        game.run();
        assert_eq!(!casts_of(&game, spell).is_empty(), offered, "{creature}");
    }
}

fn emblem(game: &mut RulesGame, name: &str, player: PlayerId) -> ObjectId {
    let card = game.cards.find(name).unwrap();
    let id = game.state.push_obj(Obj::new(card, player, Zone::COMMAND));
    game.state.zone_push(Zone::COMMAND, id);
    id
}

/// CR 114.4, 118.9: an emblem's static abilities function in the command zone, so one that gives spells an alternative
/// cost (Tamiyo, Field Researcher's "You may cast spells from your hand without paying their mana costs") or cards a way
/// to be cast (CR 604) offers that cast, made by the emblem; another player's emblem gives nothing to its owner's opponent,
/// and a card in the command zone (a commander) gives nothing.
#[test]
fn an_emblem_gives_an_alternative_cost_and_a_way() {
    let (mut game, spell) = game_with("Route Probe", Zone::hand, 0);
    assert!(casts_of(&{ game.run(); game.clone() }, spell).is_empty(), "with no mana and no emblem, nothing casts it");
    let (mut game, spell) = game_with("Route Probe", Zone::hand, 0);
    emblem(&mut game, "Route Free Emblem", PlayerId(1));
    game.run();
    assert!(casts_of(&game, spell).is_empty(), "the opponent's emblem gives its owner's spells alone");
    let (mut game, spell) = game_with("Route Probe", Zone::hand, 0);
    emblem(&mut game, "Route Free Alternative", ME);
    game.run();
    assert!(casts_of(&game, spell).is_empty(), "a card in the command zone, no emblem, gives nothing (CR 113.6)");
    let (mut game, spell) = game_with("Route Probe", Zone::hand, 0);
    let free = emblem(&mut game, "Route Free Emblem", ME);
    game.run();
    let option = casts_of(&game, spell).into_iter().find(|o| o.paid_by.is_some_and(|g| g.object == free)).expect("the emblem's free cast");
    take(&mut game, &option);
    resolve(&mut game);
    assert!(game.log.iter().any(|r| matches!(r.event, Event::SpellCast { player: ME, .. })));

    let (mut game, spell) = game_with("Route Probe", Zone::graveyard, 0);
    game.state.update_player(ME, |p| p.life = 20);
    let way = emblem(&mut game, "Route Grantor Emblem", ME);
    game.run();
    let option = casts_of(&game, spell).into_iter().find(|o| by(o) == Some(way)).expect("the emblem's way");
    take(&mut game, &option);
    resolve(&mut game);
    assert!(game.log.iter().any(|r| matches!(r.event, Event::Cast(c) if c.granted_by().is_some_and(|g| g.object == way))));
}

/// CR 601.3, 118.9: a permanent's permission to cast from exile and another's alternative cost compose; a permission that
/// names its own cost composes with none (CR 118.9a). The cast is recorded as made by the permission.
#[test]
fn a_permission_and_another_objects_alternative_cost_compose() {
    let (mut game, spell) = game_with("Route Probe", Zone::exile, 0);
    let permitter = put(&mut game, "Route Exile Permission", ME);
    let payer = put(&mut game, "Route Free Alternative", ME);
    game.run();
    let options = casts_of(&game, spell);
    let combined = options.iter().find(|o| by(o) == Some(permitter) && o.paid_by.is_some_and(|g| g.object == payer)).expect("both").clone();
    assert!(!options.iter().any(|o| o.paid_by.is_some() && by(o) != Some(permitter)));
    take(&mut game, &combined);
    resolve(&mut game);
    assert!(game.log.iter().any(|r| matches!(r.event, Event::Cast(c) if c.granted_by().is_some_and(|g| g.object == permitter))));

    let (mut game, spell) = game_with("Route Probe", Zone::exile, 0);
    let source = put(&mut game, "Route Permission Source", ME);
    put(&mut game, "Route Free Alternative", ME);
    game.state.permission_push(permission(spell, source, PlayGrant { instead: Some(ManaCost { generic: 1, ..ManaCost::ZERO }), ..PlayGrant::CAST }));
    game.state.update_player(ME, |p| p.pool.0[Mana::C as usize] = 1);
    game.run();
    assert!(casts_of(&game, spell).iter().all(|o| o.paid_by.is_none()));
}

/// CR 113.6e, 601.3: a card's own "you may cast this card from exile" functions where the card is, so the card offers that
/// cast itself rather than a permanent offering it, it offers no other card in exile, and the cast is recorded as made by the
/// card. Its controller is "you" (CR 109.5).
#[test]
fn a_card_in_exile_offers_its_own_cast_there() {
    let (mut game, spell) = game_with("Route Self Exile Probe", Zone::exile, 1);
    let other = game.state.zone(Zone::hand(ME))[0];
    move_to(&mut game, other, Zone::exile(ME));
    game.run();
    let own = casts_of(&game, spell).into_iter().find(|o| by(o) == Some(spell)).expect("the card offers its own cast");
    assert!(casts_of(&game, other).is_empty(), "a card's permission is its own, not another card's in exile");
    take(&mut game, &own);
    resolve(&mut game);
    assert!(
        game.log
            .iter()
            .any(|r| matches!(r.event, Event::Cast(c) if c.from == ZoneKind::Exile && c.granted_by().is_some_and(|g| g.object == spell)))
    );
}

/// Nine permanents giving ways: each is a cast of its own (no slot limit), the last one's cast is for its own cost (nothing)
/// with its own flashback, and the history says which permanent it was made by.
#[test]
fn every_way_a_permanent_gives_is_a_cast_of_its_own_with_its_own_terms() {
    let (mut game, spell) = game_with("Route Probe", Zone::graveyard, 0);
    let grantors: Vec<ObjectId> = (0..9).map(|n| put(&mut game, &format!("Route Grantor {n}"), ME)).collect();
    game.run();
    let options = casts_of(&game, spell);
    for &g in &grantors {
        assert_eq!(options.iter().filter(|o| by(o) == Some(g)).count(), 1);
    }
    let first = options.iter().find(|o| by(o) == Some(grantors[0])).unwrap();
    assert_eq!(cantrip_rules::granted_cast_cost(&game, ME, first).map(|c| c.life), Some(1));
    let last = options.iter().find(|o| by(o) == Some(grantors[8])).unwrap().clone();
    take(&mut game, &last);
    resolve(&mut game);
    assert_eq!(game.state.player(ME).life, 20);
    assert!(in_zone(&game, Zone::exile(ME), table().find("Route Probe").unwrap()), "the way's flashback exiles it");
    assert!(game.log.iter().any(|r| matches!(r.event, Event::Cast(c) if c.granted_by().is_some_and(|g| g.object == grantors[8]))));
}


static FUSED_PLAIN_HALF: CardDef = CardDef { name: "Route Fused Plain Half", ..PROBE_SPELL };
static FUSED_X_HALF: CardDef = CardDef { name: "Route Fused X Half", cost: ManaCost { x: 1, ..ManaCost::ZERO }, ..PROBE_SPELL };
static FUSED_MODAL_HALF: CardDef = CardDef { name: "Route Fused Modal Half", modal: Some(&FUSED_MODES), ..PROBE_SPELL };
static FUSED_MODES: ModalDef = ModalDef { modes: &[], min: 1, max: 1, escalate: None, repeat: false, allowed: None, when_paid: None, up_to_if: None, shared: None };
static FUSE_WITH_X: CardDef =
    CardDef { name: "Route Fuse With X", oracle: true, split: Some((&FUSED_X_HALF, &FUSED_PLAIN_HALF)), fuse: true, ..CardDef::BLANK };
static FUSE_WITH_MODES: CardDef =
    CardDef { name: "Route Fuse With Modes", oracle: true, split: Some((&FUSED_PLAIN_HALF, &FUSED_MODAL_HALF)), fuse: true, ..CardDef::BLANK };

/// CR 702.102, 601.2b: a fused split spell is made from its halves' targets and instructions alone, so a fuse card whose half
/// has a choice of its own as it is cast (an {X}, modes) would be cast fused without it: the table refuses to be built.
#[test]
#[should_panic(expected = "Route Fuse With X: its half Route Fused X Half has an {X}")]
fn a_fuse_card_with_an_x_half_is_refused() {
    let _ = CardTable::new([&FUSE_WITH_X, &FUSED_X_HALF, &FUSED_PLAIN_HALF]);
}

#[test]
#[should_panic(expected = "Route Fuse With Modes: its half Route Fused Modal Half has modes")]
fn a_fuse_card_with_a_modal_half_is_refused() {
    let _ = CardTable::new([&FUSE_WITH_MODES, &FUSED_MODAL_HALF, &FUSED_PLAIN_HALF]);
}
