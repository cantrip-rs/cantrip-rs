use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

fn vampire_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::subtype_card_in_your_graveyard(q, asker, t, &[Subtype::Vampire])
}

/// "Return up to one target Vampire card from your graveyard to the battlefield with a finality counter on it. You lose life
/// equal to its mana value." (CR 122.6, 614.1c, 608.2b): the card as it was in the graveyard, whose mana value an {X} in its
/// cost counts as 0 (CR 202.3e). Nothing happens, and no life is lost, when no target was chosen or the target is gone.
fn return_the_vampire_with_finality_and_lose_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let value = g.mana_value(card) as i32;
    effects::return_first_target_to_battlefield_with_counters::<{ CounterKind::Finality as usize }, 1>(g, item)?;
    g.lose_life(item.controller, value);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS_OR_ATTACKS,
        return_the_vampire_with_finality_and_lose_life,
        "Whenever this creature enters or attacks, return up to one target Vampire card from your graveyard to the battlefield with a finality counter on it. You lose life equal to its mana value.",
    )
    .targets(&[target(vampire_card_in_your_graveyard).between(0, 1)])],
    ..oracle::QUEENS_BAY_PALADIN
};
