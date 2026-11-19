use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// Chapter III: "Create a Wicked Role token attached to target creature you control" (CR 111.10q, 303.7): created attached to the
/// creature if it is still a legal target as the chapter resolves (CR 608.2b).
fn wicked_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::WICKED_ROLE, item.controller, creature)?;
    Ok(())
}

/// Chapter I destroys a creature an opponent controls whose mana value is 2 or less (CR 115.1, 202.3), judged as it is chosen and
/// again as the chapter resolves; chapter II makes a Food token; chapter III makes the Wicked Role. The lore counters, the
/// chapter triggers and the sacrifice after III are the Saga rules (CR 714).
pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, effects::destroy_first_target, "I \u{2014} Destroy target creature an opponent controls with mana value 2 or less.")
            .targets(&[target(targets::creature_an_opponent_controls_with_mana_value_at_most::<2>)]),
        chapter(2, tokens::create_a_food_token, "II \u{2014} Create a Food token."),
        chapter(3, wicked_role, "III \u{2014} Create a Wicked Role token attached to target creature you control.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    ..oracle::THE_WITCHS_VANITY
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD, &tokens::WICKED_ROLE];
