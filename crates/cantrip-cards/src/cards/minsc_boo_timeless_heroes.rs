use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_mechanics::damage_target;
use cantrip_mechanics::effects;
use cantrip_mechanics::targets;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("CreateToken", "create Boo?", "A legendary 1/1 red Hamster creature token with trample and haste", "Create Boo", "Don't");

/// Boo, a legendary 1/1 red Hamster creature token with trample and haste (CR 205.4a, 111.4).
pub static BOO: CardDef = CardDef {
    name: "Boo",
    has_cost: false,
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false, world: false },
    subtypes: SubtypeSet::of(&[Subtype::Hamster]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Trample, Keyword::Haste]),
    ..CardDef::BLANK
};

/// "You may create Boo": asked of the controller, then the token.
fn create_boo(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        tokens::create(g, item.controller, &BOO, 1)?;
    }
    Ok(())
}

/// "Up to one target creature with trample or haste", read from computed characteristics.
fn trampler_or_hasty(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if {
            let c = q.chars(id);
            c.has(Keyword::Trample) || c.has(Keyword::Haste)
        })
}

/// +1: put three +1/+1 counters on the target, if it is still legal.
fn plus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if item.target_count > 0
        && let Some(creature) = targets::object(g, item, 0)
    {
        g.add_counters(creature, CounterKind::PlusOne, 3);
    }
    Ok(())
}

/// -2: "Sacrifice a creature. When you do, Minsc & Boo deals X damage to any target, where X is that creature's power.
/// If the sacrificed creature was a Hamster, draw X cards." The creature is chosen as the ability resolves, taken
/// unasked when it is the only one; the reflexive trigger (CR 603.12) exists only if it really left, and carries the
/// creature as its subject so the power and the Hamster it last was are read from last known information (CR 608.2h).
fn minus_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let creatures: Vec<ObjectId> = g.permanents_of(player).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    let chosen = match creatures.len() {
        1 => creatures[0],
        _ => match choose::card(g, player, item.object, Purpose::Sacrifice, creatures)? {
            Some(creature) => creature,
            None => return Ok(()),
        },
    };
    if g.sacrifice(player, &[chosen]).is_empty() {
        return Ok(());
    }
    let me = g.card_id(&CARD);
    g.reflexive_trigger(item.object, player, AbilityRef::delayed(me, 0), Target::Object(chosen));
    Ok(())
}

/// The reflexive ability: damage equal to the sacrificed creature's power to any target, then, if it was a Hamster,
/// draw that many cards.
fn damage_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(sacrificed) = item.subject else { return Ok(()) };
    let chars = g.chars(sacrificed);
    let power = chars.power.unwrap_or(0).max(0) as u32;
    let hamster = chars.has_subtype(g.words(item.object).subtype(Subtype::Hamster));
    damage_target(g, item, 0, power);
    if hamster {
        g.draw(item.controller, power as u32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, create_boo, "When Minsc & Boo enters and at the beginning of your upkeep, you may create Boo, a legendary 1/1 red Hamster creature token with trample and haste."),
        triggered(triggers::YOUR_UPKEEP, create_boo, "When Minsc & Boo enters and at the beginning of your upkeep, you may create Boo, a legendary 1/1 red Hamster creature token with trample and haste."),
        ability(loyalty(1), plus_one, "+1: Put three +1/+1 counters on up to one target creature with trample or haste.")
            .targets(&[up_to_one(trampler_or_hasty)]),
        ability(loyalty(-2), minus_two, "\u{2212}2: Sacrifice a creature. When you do, Minsc & Boo deals X damage to any target, where X is that creature's power. If the sacrificed creature was a Hamster, draw X cards."),
    ],
    delayed: &[delayed_trigger(damage_and_draw, "When you do, Minsc & Boo deals X damage to any target, where X is that creature's power. If the sacrificed creature was a Hamster, draw X cards.")
        .targets(&[targets::ANY])],
    ..oracle::MINSC_BOO_TIMELESS_HEROES
};

pub static EXTRA: &[&CardDef] = &[&BOO];
