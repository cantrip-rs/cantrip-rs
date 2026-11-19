use cantrip_api::*;
use cantrip_mechanics::{bargain, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// The Role under the trigger's controller, attached to the target; nothing when the target is no longer legal (CR 608.2b).
fn create_cursed_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::CURSED_ROLE, item.controller, creature)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        create_cursed_role,
        "When this creature enters, if it was bargained, create a Cursed Role token attached to target creature an opponent controls.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])
    .condition(bargain::was_bargained)],
    ..oracle::DIMINISHER_WITCH
};

pub static EXTRA: &[&CardDef] = &[&tokens::CURSED_ROLE];
