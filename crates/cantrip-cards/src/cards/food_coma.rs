use cantrip_api::*;
use cantrip_mechanics::{exile_target_until_leaves, targets, tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Exile target creature an opponent controls until this enchantment leaves the battlefield. Create a Food token" (CR 610.3,
/// 111.10b): one ability, so with no legal target it is removed from the stack and makes no Food (CR 603.3d), and with an
/// illegal target on resolution it does nothing at all (CR 608.2b). With the enchantment already gone nothing is exiled (CR 610.3b)
/// but the Food is still created.
fn exile_until_it_leaves_and_create_a_food_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_target_until_leaves(g, item, 0);
    tokens::create_a_food_token(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        exile_until_it_leaves_and_create_a_food_token,
        "When this enchantment enters, exile target creature an opponent controls until this enchantment leaves the battlefield. Create a Food token.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::FOOD_COMA
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
