//! Which card a face's text belongs to, as linked abilities know it (CR 607.1, 607.1b; `CardTable::linked_card`): either face
//! of a transforming double-faced card is that card, a modal double-faced card's back face is a face of its own, and either
//! door of a Room is the Room. An exile made by one face's ability is found from the other face of a transforming card,
//! never across a modal card's faces. Over test cards.

use crate::{Game, Setup};
use cantrip_api::*;

/// Blue chosen for `source` as it entered, by the text `text`.
fn blue(game: &mut Game, source: ObjectId, text: CardId) {
    let choices = <cantrip_state::Choices as cantrip_state::ChoiceRecord>::entered(text, cantrip_state::Entered { color: Some(Color::Blue), ..Default::default() });
    game.state.update_obj(source, |o| o.marks.choices = choices);
}

/// A transforming card's back face has no mana cost of its own (CR 202.3b, 712.8e).
fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// The back face, with an ability of its own ("... cast a spell from among the cards exiled with this permanent", Hauken's
/// Insight's).
static TRANSFORMED: CardDef = CardDef {
    name: "Linked Test Transformed",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    has_cost: false,
    abilities: &[ability(TAP, nothing, "{T}: Back.")],
    ..CardDef::BLANK
};
/// The front face, with the ability the back's is linked to ("... exile a card from your hand face down", Jacob Hauken's).
static TRANSFORMING: CardDef = CardDef {
    name: "Linked Test Transforming",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    back: Some(&TRANSFORMED),
    abilities: &[ability(TAP, nothing, "{T}: Front.")],
    ..CardDef::BLANK
};

/// A modal double-faced card's back face with a mana cost of its own (CR 712.3: a face that may be cast).
static MODAL_BACK: CardDef = CardDef { name: "Linked Test Modal Back", types: TypeSet::of(&[CardType::Artifact]), has_cost: true, ..CardDef::BLANK };
static MODAL: CardDef = CardDef { name: "Linked Test Modal", types: TypeSet::of(&[CardType::Artifact]), has_cost: true, back: Some(&MODAL_BACK), ..CardDef::BLANK };

/// A modal double-faced card whose back face is a land played as such (CR 712.12).
static LAND_BACK: CardDef = CardDef { name: "Linked Test Land Back", types: TypeSet::of(&[CardType::Land]), has_cost: false, ..CardDef::BLANK };
static LAND_MODAL: CardDef =
    CardDef { name: "Linked Test Land Modal", types: TypeSet::of(&[CardType::Artifact]), has_cost: true, back: Some(&LAND_BACK), back_land: true, ..CardDef::BLANK };

static LEFT_DOOR: CardDef = CardDef { name: "Linked Test Left Door", types: TypeSet::of(&[CardType::Enchantment]), subtypes: SubtypeSet::of(&[Subtype::Room]), has_cost: true, ..CardDef::BLANK };
static RIGHT_DOOR: CardDef = CardDef { name: "Linked Test Right Door", types: TypeSet::of(&[CardType::Enchantment]), subtypes: SubtypeSet::of(&[Subtype::Room]), has_cost: true, ..CardDef::BLANK };
static ROOM: CardDef = CardDef {
    name: "Linked Test Room",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Room]),
    split: Some((&LEFT_DOOR, &RIGHT_DOOR)),
    ..CardDef::BLANK
};

static EXILED: CardDef = CardDef { name: "Linked Test Exiled", types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(1), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    Box::leak(Box::new(CardTable::new([&TRANSFORMING, &MODAL, &LAND_MODAL, &ROOM, &LEFT_DOOR, &RIGHT_DOOR, &EXILED])))
}

#[test]
fn each_face_belongs_to_the_card_its_text_is_printed_on() {
    let t = table();
    let id = |def: &'static CardDef| t.id_of(def);
    let back = |front: &'static CardDef| t.face(id(front), true);
    // A transforming double-faced card: both faces are one text (CR 607.1b).
    assert_ne!(back(&TRANSFORMING), id(&TRANSFORMING), "the transforming card has a back face");
    assert_eq!(t.linked_card(back(&TRANSFORMING)), id(&TRANSFORMING));
    assert_eq!(t.linked_card(id(&TRANSFORMING)), id(&TRANSFORMING));
    // A modal double-faced card's back face, whether it has a mana cost or is a land played as such, is a face of its own.
    assert_eq!(t.linked_card(back(&MODAL)), back(&MODAL));
    assert_ne!(back(&MODAL), id(&MODAL));
    assert_eq!(t.linked_card(back(&LAND_MODAL)), back(&LAND_MODAL));
    assert_ne!(back(&LAND_MODAL), id(&LAND_MODAL));
    // Each door of a Room is the Room (CR 607.1, 709.5).
    assert_eq!(t.linked_card(id(&LEFT_DOOR)), id(&ROOM));
    assert_eq!(t.linked_card(id(&RIGHT_DOOR)), id(&ROOM));
    assert_eq!(t.linked_card(id(&ROOM)), id(&ROOM));
}

/// `card` on the battlefield, transformed or not, with a card exiled by the ability of its text as it is now.
fn exiled_by(card: &'static CardDef) -> (Game, ObjectId, ObjectId) {
    let t = table();
    let mut game = Game::new(t, &Setup { decks: vec![Vec::new(); 2], ..Default::default() });
    let me = PlayerId(0);
    let source = game.state.push_obj(Obj::new(t.id_of(card), me, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, source);
    let exiled = game.state.push_obj(Obj::new(t.id_of(&EXILED), PlayerId(1), Zone::exile(PlayerId(1))));
    game.state.zone_push(Zone::exile(PlayerId(1)), exiled);
    let by = game.text_linked(source);
    game.state.update_obj(exiled, |o| o.exiled = Some(ExileLink::new(by, Zone::graveyard(PlayerId(1)), false)));
    (game, source, exiled)
}

#[test]
fn an_exile_by_one_face_is_found_from_the_other_face_of_a_transforming_card_only() {
    // Exiled by the front face's ability, found by the back face's once it transforms (CR 607.1b).
    let (mut game, source, exiled) = exiled_by(&TRANSFORMING);
    assert_eq!(game.exiled_with(game.text_linked(source)), vec![exiled]);
    game.state.update_obj(source, |o| o.transformed = true);
    assert_eq!(game.exiled_with(game.text_linked(source)), vec![exiled], "the back face's ability finds what the front face's exiled");
    // A modal double-faced card's faces are two texts: what one face's ability exiled is not the other's.
    let (mut game, source, _) = exiled_by(&MODAL);
    game.state.update_obj(source, |o| o.transformed = true);
    assert!(game.exiled_with(game.text_linked(source)).is_empty(), "the other face of a modal card finds nothing");
}

/// A transforming card's two faces are one text (CR 607.1b), but an ability of its front face an effect gives it while its
/// back face is up was acquired by that effect, and is linked to nothing it prints (CR 607.5): Agatha's Soul Cauldron lending
/// Jacob Hauken's ability to an animated Hauken's Insight. The back face's own ability is its own.
#[test]
fn a_front_face_ability_given_to_the_back_face_is_acquired_by_the_effect() {
    let (mut game, source, _) = exiled_by(&TRANSFORMING);
    let t = game.cards;
    game.state.update_obj(source, |o| o.transformed = true);
    // Given by another object's effect, as the Cauldron's.
    let lender = game.state.push_obj(Obj::new(t.id_of(&EXILED), PlayerId(0), Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, lender);
    let timestamp = game.state.timestamp();
    game.state.effect_push(ContinuousEffect {
        source: lender,
        controller: PlayerId(0),
        timestamp,
        affected: source,
        kind: EffectKind::AddAbility(AbilityRef { card: t.id_of(&TRANSFORMING), index: 0 }),
        duration: Duration::EndOfTurn,
        words: Words::NONE,
    });
    let chars = crate::layers::chars(t, &game.state, &game.log, source);
    let slot = |card: &'static CardDef| chars.abilities.iter().position(|a| a.is_some_and(|a| a.card == t.id_of(card))).expect("the ability");
    let given = crate::layers::acquired(t, &game.state, &game.log, source, &chars, slot(&TRANSFORMING));
    assert_eq!(given, Acquired { by: AcquiredBy::Grant, stamp: timestamp, part: 0 }, "the given front-face ability is the effect's");
    let printed = crate::layers::acquired(t, &game.state, &game.log, source, &chars, slot(&TRANSFORMED));
    assert_eq!(printed, Acquired::OWN, "the back face's printed ability is its own");
}

/// "+1/+0 while a color was chosen for it", read by the ability that asks (CR 607.2d): a static ability of `CHOOSER`'s text,
/// which the card prints and also lists to give (`delayed`).
fn plus_one_for_a_chosen_color(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    if q.state().chosen_color(q.linked_to(source)).is_some() {
        chars.power = chars.power.map(|p| p + 1);
    }
}

static CHOOSER: CardDef = CardDef {
    name: "Linked Test Chooser",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    statics: &[characteristic_defining(Layer::PtModify, plus_one_for_a_chosen_color, "This creature gets +1/+0 while a color was chosen.")],
    delayed: &[granted_static(Layer::PtModify, plus_one_for_a_chosen_color, "This creature gets +1/+0 while a color was chosen.")],
    ..CardDef::BLANK
};

/// A static ability an effect gives a permanent is acquired by that effect (CR 607.5), even where the permanent prints the
/// same one: linked only to the abilities that effect gave it, it reads no choice the permanent's own text made (CR 607.2d).
/// The Chooser chose a color as it entered: its printed "+1/+0" reads the choice; the same ability another object's effect
/// gave it reads none, so it is a 2/1, not a 3/1.
#[test]
fn a_static_ability_given_by_an_effect_reads_no_choice_the_permanents_own_text_made() {
    let t = Box::leak(Box::new(CardTable::new([&CHOOSER, &EXILED])));
    let mut game = Game::new(t, &Setup { decks: vec![Vec::new(); 2], ..Default::default() });
    let me = PlayerId(0);
    let source = game.state.push_obj(Obj::new(t.id_of(&CHOOSER), me, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, source);
    let text = t.linked_card(t.id_of(&CHOOSER));
    blue(&mut game, source, text);
    assert_eq!(crate::layers::chars(t, &game.state, &game.log, source).power, Some(2), "its own ability reads its own choice");
    let lender = game.state.push_obj(Obj::new(t.id_of(&EXILED), me, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, lender);
    let timestamp = game.state.timestamp();
    game.state.effect_push(ContinuousEffect {
        source: lender,
        controller: me,
        timestamp,
        affected: source,
        kind: EffectKind::AddAbility(AbilityRef::delayed(t.id_of(&CHOOSER), 0)),
        duration: Duration::EndOfTurn,
        words: Words::NONE,
    });
    let chars = crate::layers::chars(t, &game.state, &game.log, source);
    assert!(chars.abilities.contains(&Some(AbilityRef::delayed(t.id_of(&CHOOSER), 0))), "it has the given ability");
    assert_eq!(chars.power, Some(2), "the given ability reads no choice: it made none");
}

/// "Whenever a player draws a card, if a color was chosen for this, ...": the trigger event read by the ability that asks
/// (CR 607.2d).
fn drew_with_a_chosen_color(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    (matches!(event, Event::Drew { .. }) && q.state().chosen_color(q.linked_to(asker)).is_some()).then_some(Fired::default())
}

/// "Add one mana of the chosen color", read by the mana ability that asks (CR 607.2d).
fn the_chosen_color(q: &dyn Query, asker: &Asker) -> ColorSet {
    q.state().chosen_color(q.linked_to(asker)).map_or(ColorSet::NONE, ColorSet::of)
}

/// A creature that chooses a color as it enters, prints a triggered ability that reads the choice, and lists that ability
/// and a mana ability of the chosen color to give (`delayed`): the mana ability it does not print.
static TRIGGERED_CHOOSER: CardDef = CardDef {
    name: "Linked Test Triggered Chooser",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[triggered(when_asked(EventMask::DREW, drew_with_a_chosen_color), nothing, "Whenever a player draws a card, if a color was chosen, nothing.")],
    delayed: &[
        triggered(when_asked(EventMask::DREW, drew_with_a_chosen_color), nothing, "Whenever a player draws a card, if a color was chosen, nothing."),
        mana_ability(TAP, add_any_color(1), "{T}: Add one mana of the chosen color.").colors_from(the_chosen_color),
    ],
    ..CardDef::BLANK
};

/// The Triggered Chooser on the battlefield, Blue chosen as it entered by its own text, and another object's effect giving it
/// its listed ability `index` (CR 613.1f).
fn given_by_another(index: u8) -> (Game, ObjectId) {
    let t = Box::leak(Box::new(CardTable::new([&TRIGGERED_CHOOSER, &EXILED])));
    let mut game = Game::new(t, &Setup { decks: vec![Vec::new(); 2], ..Default::default() });
    let me = PlayerId(0);
    let source = game.state.push_obj(Obj::new(t.id_of(&TRIGGERED_CHOOSER), me, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, source);
    let text = t.linked_card(t.id_of(&TRIGGERED_CHOOSER));
    blue(&mut game, source, text);
    let lender = game.state.push_obj(Obj::new(t.id_of(&EXILED), me, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, lender);
    let timestamp = game.state.timestamp();
    game.state.effect_push(ContinuousEffect {
        source: lender,
        controller: me,
        timestamp,
        affected: source,
        kind: EffectKind::AddAbility(AbilityRef::delayed(t.id_of(&TRIGGERED_CHOOSER), index)),
        duration: Duration::EndOfTurn,
        words: Words::NONE,
    });
    (game, source)
}

fn cx(g: &mut Game) -> crate::Cx<'_> {
    crate::Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// A triggered ability an effect gives a permanent is acquired by that effect (CR 607.5), even where the permanent prints
/// the same one: its trigger event reads no choice the permanent's own text made (CR 607.2d). Of the Triggered Chooser's two
/// "whenever a player draws a card, if a color was chosen", only the printed one triggers.
#[test]
fn a_triggered_ability_given_by_an_effect_reads_no_choice_the_permanents_own_text_made() {
    let (mut game, source) = given_by_another(0);
    cx(&mut game).emit(Event::Drew { player: PlayerId(0), card: source });
    let triggered: Vec<Acquired> = game.state.triggers.iter().filter(|t| t.source == source).map(|t| t.acquired).collect();
    assert_eq!(triggered, vec![Acquired::OWN], "the printed ability triggers, the given one does not");
}

/// A mana ability an effect gives a permanent is acquired by that effect (CR 607.5): "one mana of the chosen color" reads no
/// choice the permanent's own text made, so it could produce no mana (CR 106.5, 106.7).
#[test]
fn a_mana_ability_given_by_an_effect_reads_no_choice_the_permanents_own_text_made() {
    let (mut game, source) = given_by_another(1);
    let (colors, colorless) = crate::actions::producible_types(&cx(&mut game), source);
    assert_eq!((colors, colorless), (ColorSet::NONE, false), "no color was chosen for the given mana ability");
}

/// An ability a permanent's own effect gives it is of its own text only if it is printed within that text (CR 607.1a);
/// one of another card's text its own effect gives it is acquired by that effect (CR 607.5, Quicksilver Elemental's example):
/// two such grants of the same card's ability, at different times, are two acquisitions, linked to nothing of each other.
#[test]
fn an_ability_a_permanents_own_effect_gives_it_is_its_own_only_if_printed_within_its_text() {
    let t = Box::leak(Box::new(CardTable::new([&TRIGGERED_CHOOSER, &TRANSFORMING, &EXILED])));
    let mut game = Game::new(t, &Setup { decks: vec![Vec::new(); 2], ..Default::default() });
    let me = PlayerId(0);
    let source = game.state.push_obj(Obj::new(t.id_of(&TRIGGERED_CHOOSER), me, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, source);
    let give = |game: &mut Game, ability: AbilityRef| {
        let timestamp = game.state.timestamp();
        game.state.effect_push(ContinuousEffect { source, controller: me, timestamp, affected: source, kind: EffectKind::AddAbility(ability), duration: Duration::EndOfTurn, words: Words::NONE });
        timestamp
    };
    // Its own text's ability, printed within the ability that gives it (CR 607.1a).
    give(&mut game, AbilityRef::delayed(t.id_of(&TRIGGERED_CHOOSER), 1));
    // Another card's, as Quicksilver Elemental gains a creature's activated abilities.
    let other = give(&mut game, AbilityRef { card: t.id_of(&TRANSFORMING), index: 0 });
    let chars = crate::layers::chars(t, &game.state, &game.log, source);
    let slot = |ability: AbilityRef| chars.abilities.iter().position(|a| *a == Some(ability)).expect("the ability");
    let own = crate::layers::acquired(t, &game.state, &game.log, source, &chars, slot(AbilityRef::delayed(t.id_of(&TRIGGERED_CHOOSER), 1)));
    assert_eq!(own, Acquired::OWN, "printed within its own text: its own");
    let acquired = crate::layers::acquired(t, &game.state, &game.log, source, &chars, slot(AbilityRef { card: t.id_of(&TRANSFORMING), index: 0 }));
    assert_eq!(acquired, Acquired { by: AcquiredBy::Grant, stamp: other, part: 0 }, "another card's, given by its own effect: the effect's");
}

/// A mana ability whose own effect reads the choice (`ManaEffect`): asked as the ability it is (CR 607.5).
fn add_if_chosen(g: &mut dyn cantrip_api::Game, asker: &Asker, color: Option<Color>) -> Result<(), Suspend> {
    if g.state().chosen_color(g.linked_to(asker)).is_some() {
        g.add_mana(asker.controller, color.unwrap_or(Color::White).mana(), 1);
    }
    Ok(())
}

static EFFECT_CHOOSER: CardDef = CardDef {
    name: "Linked Test Effect Chooser",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    delayed: &[mana_ability(TAP, add_any_color(1).by(add_if_chosen), "{T}: Add one mana of any color if a color was chosen.")],
    ..CardDef::BLANK
};

/// `card` on the battlefield under `PlayerId(0)`, Blue chosen as it entered by its own text.
fn chooser(t: &'static CardTable, game: &mut Game, card: &'static CardDef) -> ObjectId {
    let source = game.state.push_obj(Obj::new(t.id_of(card), PlayerId(0), Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, source);
    let text = t.linked_card(t.id_of(card));
    blue(game, source, text);
    source
}

/// Another object's effect gives `holder` `ability` (CR 613.1f); the effect's timestamp.
fn lend(game: &mut Game, lender: &'static CardDef, holder: ObjectId, ability: AbilityRef) -> u32 {
    let lent = game.state.push_obj(Obj::new(game.cards.id_of(lender), PlayerId(0), Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, lent);
    let timestamp = game.state.timestamp();
    game.state.effect_push(ContinuousEffect { source: lent, controller: PlayerId(0), timestamp, affected: holder, kind: EffectKind::AddAbility(ability), duration: Duration::EndOfTurn, words: Words::NONE });
    timestamp
}

/// A mana ability an effect gave a permanent runs its effect as that ability when what the permanent could produce is
/// judged (CR 106.7, 607.5): "add one mana if a color was chosen" made no choice, and could produce nothing.
#[test]
fn a_given_mana_abilitys_effect_reads_no_choice_the_permanents_own_text_made() {
    let t = Box::leak(Box::new(CardTable::new([&EFFECT_CHOOSER, &EXILED])));
    let mut game = Game::new(t, &Setup { decks: vec![Vec::new(); 2], ..Default::default() });
    let source = chooser(t, &mut game, &EFFECT_CHOOSER);
    lend(&mut game, &EXILED, source, AbilityRef::delayed(t.id_of(&EFFECT_CHOOSER), 0));
    let (colors, colorless) = crate::actions::producible_types(&cx(&mut game), source);
    assert_eq!((colors, colorless), (ColorSet::NONE, false), "the given ability made no choice, so its effect adds nothing");
}

/// "Activated abilities of this creature cost {2} more while a color was chosen for it" (Anointed Peacekeeper's tax, as a
/// color): a cost change of the permanent's own text (CR 601.2f, 602.2b).
fn tax_if_chosen(q: &dyn Query, source: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    match costed {
        Costed::Ability(id, ..) if id == source && q.state().chosen_color(q.text_linked(source)).is_some() => CostDelta::more(2),
        _ => CostDelta::NONE,
    }
}

static TAXER: CardDef = CardDef {
    name: "Linked Test Taxer",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    cost_changes: &[tax_if_chosen],
    ..CardDef::BLANK
};

static LENDER: CardDef = CardDef {
    name: "Linked Test Lender",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[ability(cost("{1}"), nothing, "{1}: Nothing.")],
    ..CardDef::BLANK
};

/// The permanent's own cost change reads its own text's choice while an ability another object's effect gave it is
/// activated (CR 607.5: the given ability's links are its own, not the permanent's other abilities'): the given {1}
/// ability costs {3}, both as the menu shows it and as the activation pays it.
#[test]
fn a_permanents_own_cost_change_reads_its_own_choice_while_a_given_ability_is_activated() {
    let t = Box::leak(Box::new(CardTable::new([&TAXER, &LENDER])));
    let mut game = Game::new(t, &Setup { decks: vec![Vec::new(); 2], ..Default::default() });
    let me = PlayerId(0);
    let source = chooser(t, &mut game, &TAXER);
    let given = AbilityRef { card: t.id_of(&LENDER), index: 0 };
    lend(&mut game, &LENDER, source, given);
    let nth = game.chars(source).abilities().position(|a| a == given).expect("given");
    let shown = crate::actions::activation_costs(&game, me, source, nth as u8, &[Target::None]);
    assert_eq!(shown.first().map(|(c, _)| c.mana_value()), Some(3), "the menu shows the tax");
    let cx = cx(&mut game);
    let by = cx.link_of_nth(source, nth);
    assert!(by.is_some(), "the ability is the effect's");
    let paid = crate::actions::apply_cost_changes_total(&cx, me, Costed::Ability(source, given, by), ManaCost { generic: 1, ..Default::default() }, &[]).0;
    assert_eq!(paid.mana_value(), 3, "the activation pays it");
}

/// "If a card would be put into an opponent's graveyard from their library, exile it instead", exiled with this permanent
/// (CR 607.2b): a replacement effect of its own text, as Valgavoth, Terror Eater's.
fn milled_to_an_opponents_graveyard(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.to.is(ZoneKind::Graveyard) && z.to.player != asker.controller && q.state().obj(z.object).zone.is(ZoneKind::Library))
}

fn exile_it_instead_with_this(q: &dyn Query, asker: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.to = Zone::exile(z.to.player);
        z.exiled_with = Some(q.linked_to(asker));
    }
}

static EXILER: CardDef = CardDef {
    name: "Linked Test Exiler",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    replacements: &[replacement(ReplacementKind::ZoneChange, milled_to_an_opponents_graveyard, exile_it_instead_with_this, "exiles a card an opponent mills")],
    ..CardDef::BLANK
};

/// "An opponent mills a card."
fn opponent_mills(g: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    g.mill(PlayerId(1), 1);
    Ok(())
}

static MILL_LENDER: CardDef = CardDef {
    name: "Linked Test Mill Lender",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[ability(TAP, opponent_mills, "{T}: An opponent mills a card.")],
    ..CardDef::BLANK
};

/// A replacement effect of the permanent's own text links what it exiles to that text (CR 607.2b) while an ability another
/// object's effect gave it resolves: the milled card is exiled with the Exiler's own text, not with the given ability.
#[test]
fn a_replacement_of_the_permanents_own_text_links_its_exile_to_that_text_while_a_given_ability_resolves() {
    let t = Box::leak(Box::new(CardTable::new([&EXILER, &MILL_LENDER, &EXILED])));
    let mut game = Game::new(t, &Setup { decks: vec![Vec::new(); 2], ..Default::default() });
    let source = game.state.push_obj(Obj::new(t.id_of(&EXILER), PlayerId(0), Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, source);
    let card = game.state.push_obj(Obj::new(t.id_of(&EXILED), PlayerId(1), Zone::library(PlayerId(1))));
    game.state.zone_push(Zone::library(PlayerId(1)), card);
    let given = AbilityRef { card: t.id_of(&MILL_LENDER), index: 0 };
    let stamp = lend(&mut game, &MILL_LENDER, source, given);
    let own = game.text_linked(source);
    let mut item = crate::actions::stack_item(StackKind::Activated, source, given, PlayerId(0), &[]);
    item.acquired = Acquired { by: AcquiredBy::Grant, stamp, part: 0 };
    let _ = opponent_mills(&mut cx(&mut game), &item);
    let exiled: Vec<ObjectId> = game.state.zone(Zone::exile(PlayerId(1))).to_vec();
    assert_eq!(exiled.len(), 1, "the milled card is exiled instead");
    assert_eq!(game.state.obj(exiled[0]).exiled.map(|l| (l.text, l.acquired)), Some((own.text, own.acquired)), "exiled with the Exiler's own text");
}
