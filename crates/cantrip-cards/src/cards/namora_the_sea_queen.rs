use cantrip_api::*;
use cantrip_mechanics::{power_up, tokens};
use cantrip_oracle as oracle;

/// The 1/1 blue Merfolk creature token.
pub static MERFOLK: CardDef = CardDef {
    name: "Merfolk Token",
    token_name: Some(&[Subtype::Merfolk]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Merfolk]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// "Put a +1/+1 counter on Namora. Create two 1/1 blue Merfolk creature tokens."
fn counter_and_merfolk(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    tokens::create(g, item.controller, &MERFOLK, 2)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[power_up::ability(
        cost("{5}{U}"),
        counter_and_merfolk,
        "Power-up \u{2014} {5}{U}: Put a +1/+1 counter on Namora. Create two 1/1 blue Merfolk creature tokens. (Activate each power-up ability only once. Reduce the cost by her mana cost if she entered this turn.)",
    )],
    ..oracle::NAMORA_THE_SEA_QUEEN
};

pub static EXTRA: &[&CardDef] = &[&MERFOLK];
