use cantrip_api::*;
use cantrip_mechanics::{exile_target, targets, tokens};
use cantrip_oracle as oracle;

/// As long as The Wandering Emperor entered this turn, you may activate her loyalty abilities any time you could cast
/// an instant (CR 606.3). The usual one loyalty activation per turn still applies.
fn instant_loyalty(q: &dyn Query, asker: &Asker, source: ObjectId, index: u8, activator: PlayerId) -> bool {
    source == asker.source
        && activator == asker.controller
        && cantrip_mechanics::history::entered_this_turn(q.history(), source)
        && q.chars(source).abilities().nth(index as usize).and_then(|a| q.activated_ability(a)).is_some_and(|d| d.is_loyalty_ability())
}

/// +1: put a +1/+1 counter on up to one target creature. It gains first strike until end of turn.
fn plus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.add_counters(creature, CounterKind::PlusOne, 1);
        g.add_effect(item.object, creature, EffectKind::AddKeywords(Keywords::of(&[Keyword::FirstStrike])), Duration::EndOfTurn);
    }
    Ok(())
}

/// A 2/2 white Samurai creature token with vigilance.
pub static SAMURAI: CardDef = CardDef {
    name: "Samurai Token",
    token_name: Some(&[Subtype::Samurai]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Samurai]),
    power: Some(2),
    toughness: Some(2),
    keywords: Keywords::of(&[Keyword::Vigilance]),
    text: "Vigilance",
    ..CardDef::BLANK
};

/// -1: create a 2/2 white Samurai creature token with vigilance.
fn minus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &SAMURAI, 1)?;
    Ok(())
}

/// -2: exile target tapped creature. You gain 2 life.
fn minus_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_target(g, item, 0);
    g.gain_life(item.controller, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(loyalty(1), plus_one, "+1: Put a +1/+1 counter on up to one target creature. It gains first strike until end of turn.")
            .targets(&[targets::CREATURE.optional()]),
        ability(loyalty(-1), minus_one, "\u{2212}1: Create a 2/2 white Samurai creature token with vigilance."),
        ability(loyalty(-2), minus_two, "\u{2212}2: Exile target tapped creature. You gain 2 life.")
            .targets(&[targets::TAPPED_CREATURE]),
    ],
    instant_activation: &[instant_loyalty],
    ..oracle::THE_WANDERING_EMPEROR
};

pub static EXTRA: &[&CardDef] = &[&SAMURAI];
