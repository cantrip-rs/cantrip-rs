use cantrip_api::*;
use cantrip_mechanics::statics::{have_double_strike_and_haste, your_of_subtype};
use cantrip_mechanics::triggers::YOUR_UPKEEP;
use cantrip_mechanics::{affinity, lands, tokens};
use cantrip_oracle as oracle;

/// "Create two 1/1 colorless Sliver creature tokens" (CR 111.1, 701.7a): both at once, as one event.
fn create_two_slivers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &SLIVER, 2)?;
    Ok(())
}

/// The 1/1 colorless Sliver creature token the Hivepool makes.
static SLIVER: CardDef = CardDef {
    colors: Some(ColorSet::NONE),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Sliver)
};

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[affinity::for_creature_type::<{ lands::sub(Subtype::Sliver) }>],
    // "Slivers you control have double strike and haste" (CR 613.1f, layer 6): every Sliver its controller controls, the type read
    // through the words of this card's text (CR 612.2).
    statics: &[static_ability(
        Layer::Ability,
        your_of_subtype::<{ lands::sub(Subtype::Sliver) }>,
        have_double_strike_and_haste,
        "Slivers you control have double strike and haste.",
    )],
    abilities: &[triggered(YOUR_UPKEEP, create_two_slivers, "At the beginning of your upkeep, create two 1/1 colorless Sliver creature tokens.")],
    ..oracle::THRUMMING_HIVEPOOL
};

pub static EXTRA: &[&CardDef] = &[&SLIVER];
