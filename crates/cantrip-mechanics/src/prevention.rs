//! Prevention effects a resolving spell or ability makes (CR 615, 611.2): "prevent the next 3 damage that would be dealt to
//! any target this turn" (a shield, CR 615.7) and "prevent all combat damage that would be dealt this turn". The core
//! applies them (`EffectKind::Prevention`): ordered with the replacement effects of the same damage (CR 616.1), never to
//! damage that can't be prevented (CR 615.12), and recorded as they prevent it (`Event::DamagePrevented`). A card lists
//! the definition it uses in its `granted_preventions` and makes the effect with the index it has there.
//!
//! "The next time a source of your choice would deal damage to you this turn, prevent that damage" (CR 609.7a, 615.8) is
//! `next_time_chosen_source_would_damage_you` with `FROM_CHOSEN_SOURCE`; the rest of such an effect ("if damage is prevented
//! this way, ...", CR 615.5) is its definition's `then`, which the core runs once the damage event has been dealt.

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
pub const ANY_DAMAGE: GrantedPreventionDef = GrantedPreventionDef { hint: "prevents the damage", damage: "damage", of_it: None, applies: any, then: None, text: "" };

/// Combat damage only (CR 510.2): "prevent all combat damage that would be dealt this turn" (Fog, Holy Day, Moment's
/// Peace). The card gives its own words (`.text(..)`).
pub const COMBAT_DAMAGE: GrantedPreventionDef = GrantedPreventionDef { hint: "prevents all combat damage", damage: "combat damage", of_it: None, applies: combat, then: None, text: "" };

fn dealt_by(_: &dyn Query, _: &Asker, source: ObjectId, d: &DamageEvent) -> bool {
    d.source == source
}

/// Damage of any kind whose source is the object the effect is about (CR 615.1, 120.1): "prevent all damage that would be
/// dealt by [that creature]". Once that object has left the zone it was in it is a new object (CR 400.7), which the effect
/// is not about. Old Fat Spider Can't See Me and Gideon of the Trials (+1) print it. The card gives its own words (`.text(..)`).
pub const DAMAGE_DEALT_BY: GrantedPreventionDef = GrantedPreventionDef { hint: "prevents all damage dealt by that permanent", damage: "damage", of_it: Some("dealt by it"), applies: dealt_by, then: None, text: "" };

/// Makes the prevention effect the resolving spell or ability's card lists `index`th in its `granted_preventions`, of damage
/// to `to` (`Target::None` for damage to anyone), until end of turn: all of it, or the next N (CR 615.7).
pub fn this_turn(g: &mut dyn Game, item: &StackItem, to: Target, index: u8, shield: Shield) {
    let affected = to.object().unwrap_or(item.object);
    // The definition is the card's whose spell or ability this is: the face that is up, if it is a permanent's.
    let card = item.ability.card;
    g.add_effect(item.object, affected, EffectKind::Prevention { card, index, to, shield }, Duration::EndOfTurn);
}

/// "Prevent all combat damage that would be dealt this turn" (CR 615.1): Fog, Holy Day, Moment's Peace. The card lists
/// `COMBAT_DAMAGE` `index`th in its `granted_preventions`.
pub fn all_combat_damage_this_turn(g: &mut dyn Game, item: &StackItem, index: u8) {
    this_turn(g, item, Target::None, index, Shield::All);
}

/// "Prevent the next N damage that would be dealt to [the target in `slot`] this turn" (CR 615.7): a shield on it, if it is
/// still a legal target (CR 608.2b). Healing Salve, Bandage, Mending Hands. The card lists `ANY_DAMAGE` `index`th in its
/// `granted_preventions`.
pub fn next_damage_to_target(g: &mut dyn Game, item: &StackItem, slot: usize, index: u8, n: u16) {
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
        let (about_it, names_source) = match e.kind {
            EffectKind::Replacement { card, index } => (true, q.names_its_source(Waiting::Replacement(card, index))),
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
/// `GrantedPreventionDef { then: Some(..), ..FROM_CHOSEN_SOURCE }`. Each card gives its own words (`text`).
pub const FROM_CHOSEN_SOURCE: GrantedPreventionDef = GrantedPreventionDef {
    hint: "prevents the damage from the chosen source",
    damage: "damage from the chosen source",
    of_it: None,
    applies: from_chosen_source,
    then: None,
    text: "",
};

/// "The next time a source of your choice would deal damage to you this turn, prevent that damage" (CR 609.7a, 615.8): the
/// spell or ability's controller chooses a source as it resolves (`choose_source`, any source), and the effect watches for the
/// next damage event in which it would deal damage to them - all of that damage is prevented, and the effect is used up
/// once the event has been dealt. The card lists `FROM_CHOSEN_SOURCE` (or a definition of the same damage with its own
/// `then`) `index`th in its `granted_preventions`. Deflecting Palm, Haazda Shield Mate, Invulnerability, Reverse Damage,
/// Intervention Pact. The source chosen, if any.
pub fn next_time_chosen_source_would_damage_you(g: &mut dyn Game, item: &StackItem, index: u8) -> Result<Option<ObjectId>, Suspend> {
    next_time_source_would_damage_you(g, item, index, SOURCE)
}

/// `next_time_chosen_source_would_damage_you` for a source `rule` allows ("a black or red source of your choice": `SOURCE`
/// with `source_of_colors` as a part), judged as it is chosen (CR 609.7a) and again, by the card's `applies`
/// (`from_chosen_source_of_colors`), as it would deal damage (CR 609.7b). Circle of Protection: Black and Greater Realm of
/// Preservation print it.
pub fn next_time_source_would_damage_you(g: &mut dyn Game, item: &StackItem, index: u8, rule: Selection) -> Result<Option<ObjectId>, Suspend> {
    let Some(source) = choose_source(g, item, rule)? else { return Ok(None) };
    let to = Target::Player(item.controller);
    g.add_effect(item.object, source, EffectKind::Prevention { card: item.ability.card, index, to, shield: Shield::NextTime }, Duration::EndOfTurn);
    Ok(Some(source))
}

/// A source with the color named by the resolving source's text (CR 609.7b, 612.2).
/// Circle of Protection: White and Circle of Protection: Blue print this choice.
fn source_of_color(q: &dyn Query, choice: &cantrip_api::select::Choice, color: u32) -> cantrip_api::select::Verdict {
    let colors = ColorSet::of(Color::ALL[color as usize]);
    cantrip_api::select::Verdict::each(choice, |id| is_of_colors(q, choice.source, id, colors))
}

/// Damage from the chosen source while it has the color named by the effect (CR 609.7b).
pub fn by_chosen_source_of_color<const COLOR: u8>(q: &dyn Query, asker: &Asker, chosen: ObjectId, damage: &DamageEvent) -> bool {
    by_chosen_source(q, chosen, damage)
        && is_of_colors(q, asker.source, damage.source, ColorSet::of(Color::ALL[COLOR as usize]))
}

/// Choose a source of the named color and prevent its next damage to you this turn (CR 609.7a-b, 615.8).
/// Circle of Protection: White and Circle of Protection: Blue print this recipe.
pub fn next_time_colored_source_would_damage_you<const COLOR: u8>(g: &mut dyn Game, item: &StackItem, index: u8) -> Result<(), Suspend> {
    let kind = match g.words(item.object).color(Color::ALL[COLOR as usize]) {
        Color::White => "white source of damage",
        Color::Blue => "blue source of damage",
        Color::Black => "black source of damage",
        Color::Red => "red source of damage",
        Color::Green => "green source of damage",
    };
    let rule = Selection::new(Count::AsManyAsCan(1))
        .with(crate::select::part(&(source_of_color as SelectFn), COLOR as u32, ""))
        .called(kind);
    let sources = damage_sources(g);
    let choice = cantrip_api::select::Choice { chooser: item.controller, source: item.object, candidates: &sources, selected: &[] };
    if !cantrip_api::select::judge(g, &rule, &choice).addable.any() {
        return Ok(());
    }
    next_time_source_would_damage_you(g, item, index, rule).map(|_| ())
}

/// Whether `id` is one of `wanted` colors as the text of `source` words them (CR 612).
fn is_of_colors(q: &dyn Query, source: ObjectId, id: ObjectId, wanted: ColorSet) -> bool {
    let colors = q.chars(id).colors;
    Color::ALL.into_iter().any(|c| wanted.has(c) && colors.has(q.words(source).color(c)))
}

/// A `SelectFn` for "a [color] source of your choice": a source of one of the colors `COLORS`, in the words of the effect's text.
pub fn source_of_colors<const COLORS: u8>(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| is_of_colors(q, choice.source, id, ColorSet(COLORS)))
}

/// A `GrantedPreventionDef::applies` for "the next time a [color] source of your choice would deal damage to you": damage from
/// the source chosen (`by_chosen_source`), as long as it is still one of the colors `COLORS` (CR 609.7b).
pub fn from_chosen_source_of_colors<const COLORS: u8>(q: &dyn Query, asker: &Asker, affected: ObjectId, d: &DamageEvent) -> bool {
    by_chosen_source(q, affected, d) && is_of_colors(q, asker.source, d.source, ColorSet(COLORS))
}
