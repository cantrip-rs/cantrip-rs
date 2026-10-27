//! Abilities the rules give an object because of what it is rather than what it prints: a basic land type's mana
//! ability (CR 305.6). Held as a card of their own in every `CardTable`, so an `AbilityRef` names them as it names
//! any other ability.

use cantrip_state::*;

use crate::ability::*;
use crate::card::*;
use crate::game::{Game, Query, Suspend};
use crate::{Event, EventMask};

fn draw_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

fn crown_subject(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Player(p) = item.subject {
        g.become_monarch(p);
    }
    Ok(())
}

fn venture(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.venture(item.controller)
}

fn initiative_subject(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Player(p) = item.subject {
        g.take_initiative(p);
    }
    Ok(())
}

const fn inherent(text: &'static str, effect: Effect) -> AbilityDef {
    delayed_trigger(effect, text)
}

/// The rules' own triggered abilities, which no object has and which the rules themselves put on the stack
/// (CR 725.2, 726.2). The first five abilities are the basic land types' mana abilities.
pub const MONARCH_DRAW: u8 = 5;
pub const MONARCH_TAKEN: u8 = 6;
pub const INITIATIVE_VENTURE: u8 = 7;
pub const INITIATIVE_TAKEN: u8 = 8;
/// A Siege's defeat ability (CR 310.12b).
pub const SIEGE_DEFEAT: u8 = 9;
/// Suspend's two triggered abilities (CR 702.62a): the upkeep one and the one for the last counter removed.
pub const SUSPEND_UPKEEP: u8 = 10;
pub const SUSPEND_CAST: u8 = 11;
/// The unlock special actions of a Room's locked doors (CR 709.5e): the rules give one for each locked door.
pub const UNLOCK_LEFT: u8 = 12;
pub const UNLOCK_RIGHT: u8 = 13;
/// Decayed's triggered ability (CR 702.147a), which the layers give a creature that has decayed once layer 6 is done, and
/// the delayed one it sets up.
pub const DECAYED: u8 = 14;
pub const DECAYED_SACRIFICE: u8 = 15;
/// The triggered ability a player with speed has (CR 702.179d).
pub const SPEED_UP: u8 = 16;
/// "The controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step": the delayed
/// triggered ability `CardDef::untimely_cleanup_rider` creates (Necromancy's cycle).
pub const UNTIMELY_SACRIFICE: u8 = 17;
/// The highest speed, max speed (CR 702.179e): "once your speed reaches 4 ... [you] can't increase it any higher" (the
/// Aetherdrift release notes).
pub const MAX_SPEED: u8 = 4;

/// "If your speed is less than 4" (CR 702.179d): the intervening "if" (CR 603.4), asked as it triggers and again as it
/// resolves. "You" is the ability's controller, the player whose speed it is.
fn speed_below_max(q: &dyn Query, asker: &Asker) -> bool {
    q.state().player(asker.controller).speed.is_some_and(|s| s < MAX_SPEED)
}

/// "Your speed increases by 1" (CR 702.179d).
fn speed_up(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.increase_speed(item.controller, 1);
    Ok(())
}

/// "When this creature attacks" (CR 508.3a).
fn this_attacks(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if *attacker == source => Fired::PLAIN,
        _ => None,
    }
}

/// "When this creature attacks, sacrifice it at end of combat" (CR 702.147a): "at end of combat" is at the beginning of the
/// end of combat step (CR 511.2), the next one, which is this combat's.
fn decayed_attacks(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let ability = AbilityRef { card: g.card_id(&INTRINSIC), index: DECAYED_SACRIFICE };
    g.delayed_trigger(DelayedWhen::Step { step: Step::EndOfCombat, on_turn_of: None }, item.object, item.controller, ability, Target::None);
    Ok(())
}

/// "Sacrifice it": the creature that attacked, if it is still that permanent (CR 400.7) and this delayed ability's
/// controller (CR 603.7e: the attack trigger's) still controls it (CR 701.21a).
fn decayed_sacrifice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.sacrifice(item.controller, &[item.object]);
    Ok(())
}

/// "The controller of the permanent it becomes sacrifices it": the permanent the delayed ability is about, if it is still that
/// object on the battlefield (CR 603.7c, 400.7), sacrificed by whoever controls it as the ability resolves (CR 701.21a), not
/// by the ability's controller.
fn untimely_sacrifice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(it) = item.subject
        && crate::game::is_live(g, it)
        && g.is_on_battlefield(it)
    {
        let controller = g.controller(it);
        g.sacrifice(controller, &[it]);
    }
    Ok(())
}

/// "A player who controls a permanent that has one or more locked halves may pay the mana cost of a locked half of that
/// permanent to give that permanent the appropriate unlocked designation" (CR 709.5e). The cost is the door's own, which
/// the rules read off the permanent (`actions::special_action`), not off this ability.
fn unlock_left(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.unlock_door(item.object, DOOR_LEFT, item.controller);
    Ok(())
}

fn unlock_right(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.unlock_door(item.object, DOOR_RIGHT, item.controller);
    Ok(())
}

fn left_is_locked(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).marks.doors & DOOR_LEFT == 0
}

fn right_is_locked(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).marks.doors & DOOR_RIGHT == 0
}

const fn unlock(text: &'static str, effect: Effect, condition: Condition) -> AbilityDef {
    ability(Cost::NONE, effect, text)
        .sorcery_speed()
        .condition(condition)
        .special_action()
}

/// "At the beginning of your upkeep, if this card is suspended, remove a time counter from it." (CR 702.62a)
fn suspend_upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = item.object;
    if !crate::game::is_live(g, card) || !g.obj(card).zone.is(ZoneKind::Exile) || g.obj(card).counters(CounterKind::Time) <= 0 {
        return Ok(());
    }
    g.remove_counters(card, CounterKind::Time, 1);
    // "When the last is removed" is a triggered ability of its own, put on the stack the next time a player gets priority.
    if g.obj(card).counters(CounterKind::Time) == 0 {
        let ability = AbilityRef { card: g.card_id(&INTRINSIC), index: SUSPEND_CAST };
        g.reflexive_trigger(card, item.controller, ability, Target::None);
    }
    Ok(())
}

/// "When the last is removed and the card is exiled, you may cast it without paying its mana cost. If you don't, it
/// remains exiled." (CR 702.62a)
fn suspend_cast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = item.object;
    if !crate::game::is_live(g, card) || !g.obj(card).zone.is(ZoneKind::Exile) || !g.could_cast_during_resolution(card) {
        return Ok(());
    }
    if g.ask(item.controller, RequestKind::May { source: card, purpose: Purpose::CastFree })? == Answer::Yes
        && g.cast_during_resolution(card, Some(ManaCost::ZERO))?
        && let Some(spell) = g.obj(card).became
        && g.obj(spell).zone.is(ZoneKind::Stack)
        && g.chars(spell).is(CardType::Creature)
    {
        // "If you cast a creature spell this way, it gains haste until you lose control of the spell or the permanent it
        // becomes" (CR 702.62a): an effect on the spell, which goes on applying to the permanent it becomes (CR 400.7a,
        // `ops::carry_effects`), for as long as its caster controls it (`SUSPEND_HASTE`).
        let lasting = Duration::AsLongAs(g.card_id(&INTRINSIC), SUSPEND_HASTE);
        g.add_effect(spell, spell, EffectKind::AddKeywords(Keywords::of(&[Keyword::Haste])), lasting);
    }
    Ok(())
}

/// The `lasting` condition of suspend's haste (CR 702.62a): the effect's controller - the spell's caster - still controls
/// the spell or the permanent it became. Once they don't, it is over for good, though control come back (CR 611.2b). The
/// spell becoming the permanent is no loss of control: the effect is carried to it as it resolves (`carry_effects`), and an
/// effect on an object that has left for anywhere else applies to nothing (CR 400.7).
pub const SUSPEND_HASTE: u8 = 0;

fn caster_controls(q: &dyn Query, e: &ContinuousEffect) -> bool {
    q.obj(e.affected).controller == e.controller
}

/// "When the last defense counter is removed from this permanent, exile it, then you may cast it transformed without
/// paying its mana cost" (CR 310.12b). "It" is the permanent the ability came from; if that has left the battlefield
/// since, it is a new object and nothing is exiled. Declined, or impossible to cast, the card stays in exile.
fn siege_defeated(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !crate::game::is_live(g, item.object) || !g.is_on_battlefield(item.object) {
        return Ok(());
    }
    let card = g.exile(item.object);
    if !g.obj(card).zone.is(ZoneKind::Exile) || !g.could_cast_transformed_during_resolution(card) {
        return Ok(());
    }
    if g.ask(item.controller, RequestKind::May { source: card, purpose: Purpose::CastTransformedFree })? == Answer::Yes {
        g.cast_transformed_during_resolution(card)?;
    }
    Ok(())
}

/// Index `Mana::W as u8` and on: "{T}: Add {W}." for Plains, and so on (CR 305.6).
pub static INTRINSIC: CardDef = CardDef {
    name: "(rules)",
    has_cost: false,
    abilities: &[
        mana_ability(TAP, add(Mana::W), "{T}: Add {W}."),
        mana_ability(TAP, add(Mana::U), "{T}: Add {U}."),
        mana_ability(TAP, add(Mana::B), "{T}: Add {B}."),
        mana_ability(TAP, add(Mana::R), "{T}: Add {R}."),
        mana_ability(TAP, add(Mana::G), "{T}: Add {G}."),
        inherent("At the beginning of the monarch's end step, that player draws a card.", draw_one),
        inherent("Whenever a creature deals combat damage to the monarch, its controller becomes the monarch.", crown_subject),
        inherent("Whenever a player takes the initiative, that player ventures into Undercity.", venture),
        inherent(
            "Whenever one or more creatures a player controls deal combat damage to the player who has the initiative, the controller of those creatures takes the initiative.",
            initiative_subject,
        ),
        inherent("When the last defense counter is removed from this permanent, exile it. Then you may cast it transformed without paying its mana cost.", siege_defeated),
        inherent("Suspend: at the beginning of your upkeep, if this card is suspended, remove a time counter from it.", suspend_upkeep),
        inherent("Suspend: when the last time counter is removed from this card, if it's exiled, you may cast it without paying its mana cost.", suspend_cast),
        unlock("Unlock the left door by paying its mana cost.", unlock_left, left_is_locked),
        unlock("Unlock the right door by paying its mana cost.", unlock_right, right_is_locked),
        triggered(when(EventMask::ATTACKS, this_attacks), decayed_attacks, "When this creature attacks, sacrifice it at end of combat."),
        inherent("Decayed: sacrifice this creature at end of combat.", decayed_sacrifice),
        inherent(
            "Whenever one or more opponents lose life during your turn, if your speed is less than 4, your speed increases by 1. This ability triggers only once each turn.",
            speed_up,
        )
        .condition(speed_below_max),
        inherent("At the beginning of the next cleanup step, the controller of the permanent it became sacrifices it.", untimely_sacrifice),
    ],
    lasting: &[Lasting { text: "you control {source}", holds: caster_controls }],
    ..CardDef::BLANK
};

/// "Turn it face up" (CR 701.40b, 701.58b, 708.7): a face-down permanent that is a creature card may be turned face up any
/// time its controller has priority, by paying its mana cost. A special action (CR 116.2): no stack. The cost is the card's
/// own, which the rules read off the permanent (`actions::special_action`), not off this ability.
fn turn_face_up(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.turn_face_up(item.object, item.controller);
    Ok(())
}

/// Whether the face-down permanent is a manifested or cloaked creature card with a mana cost that can be paid to turn it face
/// up (CR 701.40b, 701.58b).
fn can_turn_face_up(q: &dyn Query, asker: &Asker) -> bool {
    let obj = q.obj(asker.source);
    let def = q.card(obj.card);
    obj.face_down && def.types.has(CardType::Creature) && def.has_cost
}

/// "Turn it face up for its morph cost" (CR 702.37e, 116.2b): the special action any face-down permanent its controller would
/// have a morph ability on if it were face up may be turned up by - cast face down by morph, or manifested, cloaked or turned
/// face down otherwise (CR 701.40c, 701.58c). The cost is the morph cost it would have face up (`Query::turn_up_way`),
/// which the rules read off the permanent (`actions::special_action`). A megamorph cost is a morph cost; turned up for it, the
/// permanent gets a +1/+1 counter as it is turned face up (CR 702.37b, 708.11).
fn turn_face_up_for_morph(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let megamorph = g.turn_up_way(g.obj(item.object), TurnUp::Morph).is_some_and(|w| w.kind == WayKind::Megamorph);
    g.turn_face_up_as(item.object, item.controller, megamorph);
    Ok(())
}

/// "Turn it face up for its disguise cost" (CR 702.168d, 116.2b), as morph's (CR 701.40d, 701.58d).
fn turn_face_up_for_disguise(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.turn_face_up_as(item.object, item.controller, false);
    Ok(())
}

/// Whether the face-down permanent would have a morph ability if it were face up (CR 702.37e: "if the permanent wouldn't have
/// a morph cost if it were face up, it can't be turned face up this way").
fn has_morph(q: &dyn Query, asker: &Asker) -> bool {
    let obj = q.obj(asker.source);
    obj.face_down && q.turn_up_way(obj, TurnUp::Morph).is_some()
}

/// Whether the face-down permanent would have a disguise ability if it were face up (CR 702.168d).
fn has_disguise(q: &dyn Query, asker: &Asker) -> bool {
    let obj = q.obj(asker.source);
    obj.face_down && q.turn_up_way(obj, TurnUp::Disguise).is_some()
}

/// The index of "turn this permanent face up" by its mana cost among the abilities of the manifested and cloaked values
/// (`FACE_DOWN`, `CLOAKED`).
pub const TURN_FACE_UP: u8 = 0;

/// The text of the special action to turn a manifested or cloaked permanent face up for its mana cost, by which
/// `CardTable::turn_up` knows it.
pub const TURN_UP_TEXT: &str = "Turn this permanent face up by paying its mana cost.";

/// The text of the special action to turn a face-down permanent face up for its morph cost (CR 702.37e).
pub const TURN_UP_MORPH_TEXT: &str = "Turn this permanent face up by paying its morph cost.";

/// The text of the special action to turn a face-down permanent face up for its disguise cost (CR 702.168d).
pub const TURN_UP_DISGUISE_TEXT: &str = "Turn this permanent face up by paying its disguise cost.";

const TURN_UP: AbilityDef = ability(Cost::NONE, turn_face_up, TURN_UP_TEXT)
    .condition(can_turn_face_up)
    .special_action();

const TURN_UP_MORPH: AbilityDef = ability(Cost::NONE, turn_face_up_for_morph, TURN_UP_MORPH_TEXT)
    .condition(has_morph)
    .special_action();

const TURN_UP_DISGUISE: AbilityDef = ability(Cost::NONE, turn_face_up_for_disguise, TURN_UP_DISGUISE_TEXT)
    .condition(has_disguise)
    .special_action();

/// What a manifested face-down permanent is (CR 701.40a, 708.2a): a 2/2 creature with no name, no mana cost, no color and no
/// subtypes. A face-down permanent has this as its copiable values (CR 707.2). It has no abilities: the special actions the
/// rules give it to turn it face up are listed here (CR 116.2b; for its mana cost, CR 701.40b, and for its morph or disguise
/// cost, CR 701.40c-d), and the layers give them back where an effect took its abilities (`layers::turn_face_up_actions`).
pub static FACE_DOWN: CardDef = CardDef {
    name: "",
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    abilities: &[TURN_UP, TURN_UP_MORPH, TURN_UP_DISGUISE],
    face_down: true,
    ..CardDef::BLANK
};

/// What a cloaked permanent is (CR 701.58a): a 2/2 creature with ward {2}, no name, no mana cost, no color and no
/// subtypes, turned face up as a manifested one is (CR 701.58b-d).
pub static CLOAKED: CardDef = CardDef {
    name: "",
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    text: "Ward {2}",
    abilities: &[TURN_UP, TURN_UP_MORPH, TURN_UP_DISGUISE, ward::<2>("Ward {2}")],
    face_down: true,
    ..CardDef::BLANK
};

/// What a card cast face down by morph is (CR 702.37a, 702.37c, 708.2): a 2/2 creature with no text, no name, no subtypes
/// and no mana cost, as a spell and as the permanent it becomes. Turned face up for its morph cost, or its disguise cost
/// (CR 702.37e, 702.168d), never for its mana cost: it was not manifested.
pub static MORPHED: CardDef = CardDef {
    name: "",
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    abilities: &[TURN_UP_MORPH, TURN_UP_DISGUISE],
    face_down: true,
    ..CardDef::BLANK
};

/// What a card cast face down by disguise is (CR 702.168a-b): a 2/2 creature with ward {2}, no name, no subtypes and no mana
/// cost, as a spell and as the permanent it becomes; turned face up for its disguise cost, or its morph cost (CR 702.168d,
/// 702.37e).
pub static DISGUISED: CardDef = CardDef {
    name: "",
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    text: "Ward {2}",
    abilities: &[TURN_UP_MORPH, TURN_UP_DISGUISE, ward::<2>("Ward {2}")],
    face_down: true,
    ..CardDef::BLANK
};

/// "Pay {cost}, or it is countered?": the question a "counter it unless that player pays" asks (CR 118.12).
pub const COUNTERED_UNLESS_PAID: Question =
    Question::new("Countered", "pay {cost}, or {subject} is countered?", "", "Pay {cost}", "Let it be countered");

/// Ward's trigger event (CR 702.21a): this permanent became the target of a spell or ability an opponent controls. Its
/// subject is what targeted it, and its data who controls that and where it is on the stack.
pub fn ward_matches(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::BecameTarget { target: Target::Object(t), by, controller } if *t == source && *controller != q.controller(source) => {
            // What targeted it, who controls that, and where it is on the stack (it is the top item as it is announced).
            Some(Fired {
                subject: Target::Object(*by),
                data: [controller.0 as u32, q.state().stack.len().saturating_sub(1) as u32],
            })
        }
        _ => None,
    }
}

fn ward_effect<const N: u8>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(by) = item.subject else { return Ok(()) };
    let payer = PlayerId(item.data[0] as u8);
    if !g.may_pay(payer, item.object, by, Cost { mana: generic(N), ..Cost::NONE }, &COUNTERED_UNLESS_PAID)? {
        g.counter_item(by, item.data[1] as usize);
    }
    Ok(())
}

/// Ward {N} (CR 702.21a): "Whenever this becomes the target of a spell or ability an opponent controls, counter it
/// unless that player pays {N}." Here because a cloaked permanent has it by the rules (CR 701.58a); the cards that print
/// or grant it reach it as `cantrip_mechanics::triggers::ward`.
pub const fn ward<const N: u8>(text: &'static str) -> AbilityDef {
    triggered(when(EventMask::BECAME_TARGET, ward_matches), ward_effect::<N>, text).ward()
}

/// What a face-down card in a zone other than the battlefield and the stack is (CR 406.3a): no characteristics at all - no
/// name, no mana cost, no color, no types, no abilities, no power or toughness. Who may look at it sees the card itself
/// (CR 406.3), which is a matter of what they know, not of what it is.
pub static NO_CHARACTERISTICS: CardDef = CardDef { name: "", has_cost: false, ..CardDef::BLANK };
