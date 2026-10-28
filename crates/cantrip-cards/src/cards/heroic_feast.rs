use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ENTERS, YOU_GAIN_LIFE};
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "When this enchantment enters, create a Food token." (CR 111.10b)
fn create_food(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_food(g, item.controller)
}

/// "Whenever you gain life, choose up to that many target creatures you control. Put a +1/+1 counter on each of them." (CR 119.3,
/// 603.2, 601.2c, 122.1): the targets are chosen as the ability goes on the stack, up to the amount of life gained
/// (`counted_by_amount`); each one still legal as it resolves gets a counter, all at once.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, create_food, "When this enchantment enters, create a Food token."),
        triggered(
            YOU_GAIN_LIFE,
            effects::plus_one_counter_on_each_target,
            "Whenever you gain life, choose up to that many target creatures you control. Put a +1/+1 counter on each of them.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL.between(0, TargetSpec::COUNTED)])
        .counted_by_amount(),
    ],
    ..oracle::HEROIC_FEAST
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
