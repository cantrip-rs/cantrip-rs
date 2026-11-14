use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::{THIS_ENTERS, THIS_LEAVES}};
use cantrip_oracle as oracle;

/// Stangg Twin, the legendary 3/4 red and green Human Warrior token (CR 111.4).
pub static TWIN: CardDef = CardDef {
    name: "Stangg Twin",
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Human, Subtype::Warrior]),
    supertypes: Supertypes { legendary: true, basic: false, snow: false, world: false },
    colors: Some(ColorSet(ColorSet::of(Color::Red).0 | ColorSet::of(Color::Green).0)),
    has_cost: false,
    power: Some(3),
    toughness: Some(4),
    ..CardDef::BLANK
};

/// "Exile that token when Stangg leaves the battlefield." (CR 603.7c)
fn exile_twin(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let twins = g.event_objects(item).to_vec();
    g.exile_all(&twins);
    Ok(())
}

/// "When Stangg enters, create Stangg Twin ..." (CR 701.7a, 603.7a)
fn enter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let twins = tokens::create(g, item.controller, &TWIN, 1)?;
    g.end_sentence();
    g.delayed_trigger_about(DelayedWhen::WhenEvent, item.object, item.controller, AbilityRef::delayed(item.ability.card, 0), &twins);
    g.end_sentence();
    g.delayed_trigger_about(DelayedWhen::AnyOfObjectsLeaves, item.object, item.controller, AbilityRef::delayed(item.ability.card, 1), &twins);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, enter, "When Stangg enters, create Stangg Twin, a legendary 3/4 red and green Human Warrior creature token. Exile that token when Stangg leaves the battlefield. Sacrifice Stangg when that token leaves the battlefield.")],
    delayed: &[
        triggered(THIS_LEAVES, exile_twin, "Exile that token when Stangg leaves the battlefield."),
        delayed_trigger(cantrip_mechanics::effects::sacrifice_self, "Sacrifice Stangg when that token leaves the battlefield."),
    ],
    ..oracle::STANGG
};
pub static EXTRA: &[&CardDef] = &[&TWIN];
