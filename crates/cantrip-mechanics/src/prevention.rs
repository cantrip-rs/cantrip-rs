//! Prevention effects a resolving spell or ability makes (CR 615, 611.2): "prevent the next 3 damage that would be dealt to
//! any target this turn" (a shield, CR 615.7) and "prevent all combat damage that would be dealt this turn". The core
//! applies them (`EffectKind::Prevention`): ordered with the replacement effects of the same damage (CR 616.1), never to
//! damage that can't be prevented (CR 615.12), and recorded as they prevent it (`Event::DamagePrevented`). A card lists
//! the definition it uses in its `granted_preventions` and makes the effect with the index it has there.
//!
//! "The next time a source of your choice would deal damage to you this turn, prevent that damage" (CR 609.7a, 615.8) is
//! `next_time_chosen_source_would_damage_you` with `FROM_CHOSEN_SOURCE`; the rest of such an effect ("if damage is prevented
//! this way, ...", CR 615.5) is its definition's `then`, which the core runs once the damage event has been dealt - except
//! "you gain life equal to the damage prevented this way", which is part of the damage event itself (CR 120.4's Awe Strike
//! example): its definition's `gain_prevented`.

use cantrip_api::*;
use crate::select::{Count, Selection};

fn any(_: &dyn Query, _: &Asker, _: ObjectId, _: &DamageEvent) -> bool {
    true
}

fn combat(_: &dyn Query, _: &Asker, _: ObjectId, d: &DamageEvent) -> bool {
    d.combat
}

/// Damage of any kind from any source, to whom the effect names: "prevent the next 3 damage that would be dealt to any
/// target this turn" (Healing Salve; Bandage's next 1, Mending Hands' next 4).
/// The card gives its own words (`.text(..)`).
pub const ANY_DAMAGE: GrantedPreventionDef = GrantedPreventionDef { hint: "prevents the damage", damage: "damage", of_it: None, applies: any, portion: None, then: None, gain_prevented: false, text: "" };

/// Combat damage only (CR 510.2): "prevent all combat damage that would be dealt this turn" (Fog, Holy Day, Moment's
/// Peace). The card gives its own words (`.text(..)`).
pub const COMBAT_DAMAGE: GrantedPreventionDef = GrantedPreventionDef { hint: "prevents all combat damage", damage: "combat damage", of_it: None, applies: combat, portion: None, then: None, gain_prevented: false, text: "" };

fn dealt_by(_: &dyn Query, _: &Asker, source: ObjectId, d: &DamageEvent) -> bool {
    d.source == source
}

/// Damage of any kind whose source is the object the effect is about (CR 615.1, 120.1): "prevent all damage that would be
/// dealt by [that creature]". Once that object has left the zone it was in it is a new object (CR 400.7), which the effect
/// is not about. Old Fat Spider Can't See Me and Gideon of the Trials (+1) print it. The card gives its own words (`.text(..)`).
pub const DAMAGE_DEALT_BY: GrantedPreventionDef = GrantedPreventionDef { hint: "prevents all damage dealt by that permanent", damage: "damage", of_it: Some("dealt by it"), applies: dealt_by, portion: None, then: None, gain_prevented: false, text: "" };

fn combat_dealt_by(_: &dyn Query, _: &Asker, source: ObjectId, d: &DamageEvent) -> bool {
    d.combat && d.source == source
}

/// Combat damage (CR 510.2) whose source is the object the effect is about (CR 615.1): "prevent all combat damage that would be
/// dealt by target creature this turn". Horn of Deafening and Lady Evangela print it. The card gives its own words (`.text(..)`).
pub const COMBAT_DAMAGE_DEALT_BY: GrantedPreventionDef = GrantedPreventionDef { hint: "prevents all combat damage dealt by that creature", damage: "combat damage", of_it: Some("dealt by it"), applies: combat_dealt_by, portion: None, then: None, gain_prevented: false, text: "" };

fn combat_dealt_to_and_by(_: &dyn Query, _: &Asker, creature: ObjectId, d: &DamageEvent) -> bool {
    d.combat && (d.source == creature || d.target == Target::Object(creature))
}

/// Combat damage (CR 510.2) whose source or recipient is the creature the effect is about (CR 615.1): "prevent all combat
/// damage that would be dealt to and dealt by that creature this turn". Once that creature has left the battlefield it is a new
/// object (CR 400.7), which the effect is not about. Maze of Ith and Ebony Horse print it.
pub const COMBAT_DAMAGE_DEALT_TO_AND_BY: GrantedPreventionDef = GrantedPreventionDef {
    hint: "prevents all combat damage dealt to and dealt by that creature",
    damage: "combat damage",
    of_it: Some("to and dealt by it"),
    applies: combat_dealt_to_and_by,
    portion: None,
    then: None,
    gain_prevented: false,
    text: "Prevent all combat damage that would be dealt to and dealt by that creature this turn.",
};

/// "Untap target [attacking creature]. Prevent all combat damage that would be dealt to and dealt by that creature this turn"
/// (CR 701.26b, 615.1, 608.2b): the creature the resolving ability targeted in slot 0, if it is still a legal target, is untapped
/// and gets the prevention. It stays in combat, still blocked or unblocked (the rulings): the damage it would deal and be dealt
/// is prevented as it is dealt, after it is assigned (CR 510.1-2). The prevention lasts until the turn ends (CR 615.4, 514.2), and
/// none applies where damage can't be prevented (CR 615.12, the core). Maze of Ith and Ebony Horse print it; the card lists
/// `COMBAT_DAMAGE_DEALT_TO_AND_BY` first in its `granted_preventions`.
pub fn untap_target_and_prevent_combat_damage_to_and_by_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = crate::targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| g.untap(creature));
    g.end_sentence();
    g.add_effect(
        item.object,
        creature,
        EffectKind::Prevention { card: item.ability.card, index: 0, to: Target::None, shield: Shield::All },
        Duration::EndOfTurn,
    );
    Ok(())
}

/// "Prevent all combat damage that would be dealt by target creature this turn" (CR 615.1, 611.2a): the creature the resolving
/// ability targeted in slot 0, if it is still a legal target (CR 608.2b). The card lists `COMBAT_DAMAGE_DEALT_BY` first in its
/// `granted_preventions`.
pub fn all_combat_damage_dealt_by_target_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = crate::targets::object(g, item, 0) else { return Ok(()) };
    g.add_effect(
        item.object,
        creature,
        EffectKind::Prevention { card: item.ability.card, index: 0, to: Target::None, shield: Shield::All },
        Duration::EndOfTurn,
    );
    Ok(())
}

/// "Prevent all damage that would be dealt by target creature this turn. That creature gets +0/+X until end of turn, where X is
/// its mana value" (CR 615.1, 608.2h, 611.2a), for the creature the resolving ability or spell targeted in slot 0, if it is
/// still a legal target (CR 608.2b): the prevention, then (CR 608.2c) the toughness. The card lists its prevention (all damage,
/// `DAMAGE_DEALT_BY`, or all combat damage, `COMBAT_DAMAGE_DEALT_BY`) first in its `granted_preventions`. Kry Shield and Subdue
/// print it.
pub fn prevent_damage_by_target_then_toughness_equal_to_mana_value(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = crate::targets::object(g, item, 0) else { return Ok(()) };
    g.add_effect(
        item.object,
        creature,
        EffectKind::Prevention { card: item.ability.card, index: 0, to: Target::None, shield: Shield::All },
        Duration::EndOfTurn,
    );
    g.end_sentence();
    let toughness = g.mana_value(creature) as i32;
    g.add_effect(item.object, creature, EffectKind::Pump(0, toughness), Duration::EndOfTurn);
    Ok(())
}

/// Makes the prevention effect the resolving spell or ability's card lists `index`th in its `granted_preventions`, of damage
/// to `to` (`Target::None` for damage to anyone), until end of turn: all of it, or the next N (CR 615.7).
pub fn this_turn(g: &mut dyn Game, item: &StackItem, to: Target, index: u8, shield: Shield) {
    let affected = to.object().unwrap_or(item.object);
    // The definition is the card's whose spell or ability this is: the face that is up, if it is a permanent's.
    let card = item.ability.card;
    g.add_effect(item.object, affected, EffectKind::Prevention { card, index, to, shield }, Duration::EndOfTurn);
}

/// "Prevent the next 1 damage that would be dealt to this creature this turn" (CR 615.7): a shield of 1 on the permanent whose
/// ability this is, until end of turn. Balduvian Hydra (for a +1/+0 counter) and Rock Hydra (for {R}) print it; the card lists
/// `ANY_DAMAGE` first in its `granted_preventions`.
pub fn next_1_damage_to_this_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    this_turn(g, item, Target::Object(item.object), 0, Shield::Next(1));
    Ok(())
}

/// "Prevent all combat damage that would be dealt this turn" (CR 615.1): Fog, Holy Day, Moment's Peace. The card lists
/// `COMBAT_DAMAGE` `index`th in its `granted_preventions`.
pub fn all_combat_damage_this_turn(g: &mut dyn Game, item: &StackItem, index: u8) {
    this_turn(g, item, Target::None, index, Shield::All);
}

/// "Prevent the next N damage that would be dealt to [the target in `slot`] this turn" (CR 615.7): a shield on it, if it is
/// still a legal target (CR 608.2b). Healing Salve, Bandage, Mending Hands. The card lists `ANY_DAMAGE` `index`th in its
/// `granted_preventions`.
pub fn next_damage_to_target(g: &mut dyn Game, item: &StackItem, slot: usize, index: u8, n: u32) {
    if !g.target_legal(item, slot) {
        return;
    }
    let target = g.state().target(item, slot);
    this_turn(g, item, target, index, Shield::Next(n));
}

/// The sources of damage a player may choose for "a source of your choice" (CR 609.7a): every permanent; every spell on the
/// stack, and every object an object on the stack refers to - an ability's source, the objects it targets or is about -
/// even one no longer in the zone it was in; every object a replacement or prevention effect waiting to apply refers to (the
/// object it is about, and its source where its words refer to it: `Query::names_its_source`), and one a delayed triggered
/// ability waiting to trigger refers to (the same); and every face-up object in the command zone. A source need not be able to deal damage to
/// be chosen. Each once, permanents first. (A triggered ability's `StackItem::data` is numbers its card reads - a payer, a
/// count, a stack position - never an object: the objects it is about are its subject and members.)
pub fn damage_sources(q: &dyn Query) -> Vec<ObjectId> {
    let s = q.state();
    let mut sources: Vec<ObjectId> = s.battlefield().to_vec();
    let mut add = |id: ObjectId| {
        if !sources.contains(&id) {
            sources.push(id);
        }
    };
    for item in &s.stack {
        add(item.object);
        s.targets(item).iter().filter_map(|t| t.object()).for_each(&mut add);
        s.members(item.members).iter().copied().for_each(&mut add);
        if let Some(id) = item.subject.object() {
            add(id);
        }
    }
    // A waiting replacement or prevention effect refers to the object it is about, and to its source only where its text names
    // it ("Deflecting Palm deals that much damage"): a resolved Fog's "prevent all combat damage" refers to no object.
    for e in &s.effects {
        // The recipient it names besides the object it is about (Jade Monolith's target creature).
        if let EffectKind::Replacement { to: Target::Object(id), .. } = e.kind {
            add(id);
        }
        let (about_it, names_source) = match e.kind {
            EffectKind::Replacement { card, index, .. } => (true, q.names_its_source(Waiting::Replacement(card, index))),
            // About a player: its `affected` object is only its source.
            EffectKind::ReplacementRule { card, index } => (false, q.names_its_source(Waiting::Replacement(card, index))),
            // About the object damage to which it prevents, or one it names more than so (Maze of Ith's creature, the chosen
            // source); one about no object - "all combat damage", damage to a player - has its source as `affected`.
            EffectKind::Prevention { card, index, to, .. } => {
                (to.object().is_some() || e.affected != e.source, q.names_its_source(Waiting::Prevention(card, index)))
            }
            // "The next time [it] would be destroyed this turn" (CR 701.19a, 614.8): of the permanent it is about.
            EffectKind::RegenerationShield => (true, false),
            _ => continue,
        };
        if about_it {
            add(e.affected);
        }
        if names_source {
            add(e.source);
        }
    }
    // A waiting delayed triggered ability (CR 603.7): the objects it is about, and its source (CR 603.7d) only where its text
    // names it ("Dragonhawk deals", "Exile this creature"): Flickerwisp's and Otherworldly Journey's "return that card" refer
    // to the card, not the creature or the spell. The rules' own delayed abilities are asked the same: rebound's "you may cast
    // this card from exile" refers to the exiled card (CR 702.88a), the untimely cleanup sacrifice only to its permanent.
    for d in &s.delayed {
        if q.names_its_source(Waiting::Delayed(d.ability)) {
            add(d.source);
        }
        s.members(d.members).iter().copied().for_each(&mut add);
        if let Some(id) = d.subject.object() {
            add(id);
        }
    }
    for &id in s.zone(Zone::COMMAND) {
        if !s.obj(id).face_down {
            add(id);
        }
    }
    sources
}

/// "A source of your choice" (CR 609.7a): the spell or ability's controller chooses one of the sources `rule` allows among
/// `damage_sources`, as it resolves (Deflecting Palm's ruling: "You choose a source of damage as Deflecting Palm resolves");
/// it is not targeted. "A red source of your choice" is a rule with that quality, judged as the source is chosen (and again
/// as it would deal damage, by the effect's `applies`: CR 609.7b). None when there is none to choose.
pub fn choose_source(g: &mut dyn Game, item: &StackItem, rule: Selection) -> Result<Option<ObjectId>, Suspend> {
    let sources = damage_sources(&*g);
    // With no source the rule allows there is nothing to choose, so nothing is asked (CR 609.7a).
    if !cantrip_api::select::first_answer(&*g, &rule, item.controller, item.object, &sources, true).is_some_and(|chosen| !chosen.is_empty()) {
        return Ok(None);
    }
    Ok(crate::choose::select(g, item.controller, item.object, Purpose::Choose, sources, rule)?.first().copied())
}

/// Any source (`choose_source`): "a source of your choice" - of damage, as the question is worded to its player ("choose a source
/// of damage").
pub const SOURCE: Selection = Selection::new(Count::Exactly(1)).called("source of damage");

/// Whether damage is dealt by the source an effect is about, chosen as it was made (CR 609.7a): that object, or - where a
/// permanent spell was chosen - the permanent it became as it resolved, "any damage dealt by that spell and any damage dealt
/// by the permanent that spell becomes". Once the source has become another object any other way, the new object is not it
/// (CR 400.7).
pub fn by_chosen_source(q: &dyn Query, chosen: ObjectId, d: &DamageEvent) -> bool {
    if d.source == chosen {
        return true;
    }
    let was = q.obj(chosen);
    // The permanent it became, even once that has left the battlefield and deals damage as it last existed (a dies trigger's):
    // it keeps the zone it left from. What that permanent became in turn is another object.
    was.zone.is(ZoneKind::Stack) && was.became == Some(d.source) && q.obj(d.source).zone.is(ZoneKind::Battlefield)
}

fn from_chosen_source(q: &dyn Query, _: &Asker, chosen: ObjectId, d: &DamageEvent) -> bool {
    by_chosen_source(q, chosen, d)
}

/// Damage from the source the effect is about (`by_chosen_source`), to whom the effect names: "the next time a source of your
/// choice would deal damage to you this turn, prevent that damage" (Haazda Shield Mate, Invulnerability, Pentagram of the
/// Ages, Righteous Aura). A card whose effect does more (CR 615.5) gives it its `then`: Deflecting Palm's
/// `GrantedPreventionDef { then: Some(..), ..FROM_CHOSEN_SOURCE }`; one that gains the life it prevented as part of the damage
/// event (CR 120.4) sets `gain_prevented` (Reverse Damage). Each card gives its own words (`text`).
pub const FROM_CHOSEN_SOURCE: GrantedPreventionDef = GrantedPreventionDef {
    hint: "prevents the damage from the chosen source",
    damage: "damage from the chosen source",
    of_it: None,
    applies: from_chosen_source,
    portion: None,
    then: None,
    gain_prevented: false,
    text: "",
};

/// "The next time a source of your choice would deal damage to you this turn, prevent that damage" (CR 609.7a, 615.8): the
/// spell or ability's controller chooses a source as it resolves (`choose_source`, any source), and the effect watches for the
/// next damage event in which it would deal damage to them - all of that damage is prevented, and the effect is used up
/// once the event has been dealt. The card lists `FROM_CHOSEN_SOURCE` (or a definition of the same damage with its own
/// `then`, or a `portion` of it) `index`th in its `granted_preventions`. Deflecting Palm, Haazda Shield Mate, Invulnerability,
/// Reverse Damage, Intervention Pact. The source chosen, if any.
pub fn next_time_chosen_source_would_damage_you(g: &mut dyn Game, item: &StackItem, index: u8) -> Result<Option<ObjectId>, Suspend> {
    next_time_source_would_damage_you(g, item, index, SOURCE)
}

/// `next_time_chosen_source_would_damage_you` as the whole of a resolving spell or ability's effect, its card listing the
/// definition first (index 0) in its `granted_preventions`: Deflecting Palm, Reverse Damage, Pentagram of the Ages, Dark Sphere.
pub fn resolve_next_time_chosen_source_would_damage_you(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    next_time_chosen_source_would_damage_you(g, item, 0)?;
    Ok(())
}
/// `next_time_chosen_source_would_damage_you` for a source `rule` allows ("a black or red source of your choice": `SOURCE`
/// with `source_of_colors` as a part), judged as it is chosen (CR 609.7a) and again, by the card's `applies`
/// (`from_chosen_source_of_colors`), as it would deal damage (CR 609.7b). Circle of Protection: Artifacts; the colored
/// ones go through `next_time_source_of_colors_would_damage_you`.
pub fn next_time_source_would_damage_you(g: &mut dyn Game, item: &StackItem, index: u8, rule: Selection) -> Result<Option<ObjectId>, Suspend> {
    let Some(source) = choose_source(g, item, rule)? else { return Ok(None) };
    let to = Target::Player(item.controller);
    g.add_effect(item.object, source, EffectKind::Prevention { card: item.ability.card, index, to, shield: Shield::NextTime }, Duration::EndOfTurn);
    Ok(Some(source))
}

/// What a source of each set of colors (a `ColorSet`'s bits) is called in the question: "black or red source of damage".
const COLORED_SOURCES: [&str; 32] = [
    "source of damage",
    "white source of damage",
    "blue source of damage",
    "white or blue source of damage",
    "black source of damage",
    "white or black source of damage",
    "blue or black source of damage",
    "white, blue, or black source of damage",
    "red source of damage",
    "white or red source of damage",
    "blue or red source of damage",
    "white, blue, or red source of damage",
    "black or red source of damage",
    "white, black, or red source of damage",
    "blue, black, or red source of damage",
    "white, blue, black, or red source of damage",
    "green source of damage",
    "white or green source of damage",
    "blue or green source of damage",
    "white, blue, or green source of damage",
    "black or green source of damage",
    "white, black, or green source of damage",
    "blue, black, or green source of damage",
    "white, blue, black, or green source of damage",
    "red or green source of damage",
    "white, red, or green source of damage",
    "blue, red, or green source of damage",
    "white, blue, red, or green source of damage",
    "black, red, or green source of damage",
    "white, black, red, or green source of damage",
    "blue, black, red, or green source of damage",
    "white, blue, black, red, or green source of damage",
];

/// "The next time a [color] source of your choice would deal damage to you this turn, prevent that damage" for the colors
/// `COLORS`, worded in the question as the text of the ability's source words them now (CR 612: a Circle of Protection: Blue
/// whose "blue" became "red" asks for a red source): `next_time_source_would_damage_you` over `SOURCE` limited by
/// `source_of_colors`, choosing none, and making no shield, when no source has those colors. The card lists
/// `from_chosen_source_of_colors::<COLORS>` first (index 0) in its `granted_preventions`. Circle of Protection: Black and
/// Blue and Greater Realm of Preservation print it.
pub fn next_time_source_of_colors_would_damage_you<const COLORS: u8>(g: &mut dyn Game, item: &StackItem) -> Result<Option<ObjectId>, Suspend> {
    let words = g.words(item.object);
    let said = Color::ALL.into_iter().filter(|&c| ColorSet(COLORS).has(c)).fold(ColorSet::NONE, |set, c| set.with(words.color(c)));
    let rule = SOURCE
        .called(COLORED_SOURCES[usize::from(said.0 & 31)])
        .taking(crate::select::as_many_as_can(1))
        .with(crate::select::part(&(source_of_colors::<COLORS> as SelectFn), 0, ""));
    next_time_source_would_damage_you(g, item, 0, rule)
}

/// Whether `id` is one of the colors `COLORS` (a `ColorSet`'s bits) as the text of `source` words them (CR 612).
fn is_of_colors<const COLORS: u8>(q: &dyn Query, source: ObjectId, id: ObjectId) -> bool {
    let colors = q.chars(id).colors;
    Color::ALL.into_iter().any(|c| ColorSet(COLORS).has(c) && colors.has(q.words(source).color(c)))
}

/// A `SelectFn` for "a [color] source of your choice": a source of one of the colors `COLORS`, in the words of the effect's text.
pub fn source_of_colors<const COLORS: u8>(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| is_of_colors::<COLORS>(q, choice.source, id))
}

/// A `GrantedPreventionDef::applies` for "the next time a [color] source of your choice would deal damage to you": damage from
/// the source chosen (`by_chosen_source`), as long as it is still one of the colors `COLORS` (CR 609.7b).
pub fn from_chosen_source_of_colors<const COLORS: u8>(q: &dyn Query, asker: &Asker, affected: ObjectId, d: &DamageEvent) -> bool {
    by_chosen_source(q, affected, d) && is_of_colors::<COLORS>(q, asker.source, d.source)
}

/// A `PreventionDef::applies` for "prevent all damage that would be dealt to enchanted creature by sources of the chosen color"
/// (CR 615.1, 303.4m): damage dealt to the creature the Aura is attached to by a source of the color the Aura's controller
/// chose as it entered (`Entering::chosen_color`). Prismatic Ward and Chromatic Armor print it.
pub fn enchanted_creature_from_sources_of_chosen_color(q: &dyn Query, asker: &Asker, e: &DamageEvent) -> bool {
    q.state().chosen_color(q.linked_to(asker)).is_some_and(|chosen| q.chars(e.source).colors.has(chosen))
        && matches!(e.target, Target::Object(o) if q.attached_to(asker.source) == Some(o))
}
