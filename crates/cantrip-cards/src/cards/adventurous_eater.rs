use cantrip_api::*;
use cantrip_mechanics::{prepare, targets};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on target creature. You gain 1 life." A target no longer legal gets no counter, and the life is
/// gained only when the spell resolves at all (CR 608.2b).
fn have_a_bite(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.add_counters(creature, CounterKind::PlusOne, 1);
    }
    g.gain_life(item.controller, 1);
    Ok(())
}

/// Adventurous Eater's prepare spell (CR 722): not a card here, only ever a copy.
pub static HAVE_A_BITE: CardDef = CardDef {
    name: "Have a Bite",
    cost: mana("{B}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[targets::CREATURE], have_a_bite),
    text: "Put a +1/+1 counter on target creature. You gain 1 life.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    enters: Some(prepare::enters_prepared),
    prepare: Some(&HAVE_A_BITE),
    ..oracle::ADVENTUROUS_EATER
};

pub static EXTRA: &[&CardDef] = &[&HAVE_A_BITE];
