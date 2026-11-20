use cantrip_api::*;
use cantrip_mechanics::{bargain, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Add four mana in any combination of colors." (CR 106.1a): each mana's color is chosen as the ability resolves, repeats allowed,
/// in one question, and the mana stays in the pool until the step ends.
fn add_four(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for color in cantrip_mechanics::choose_colors(g, item.controller, item.object, ColorSet(0b11111), 4)? {
        g.add_mana(item.controller, color.mana(), 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    abilities: &[
        triggered(triggers::THIS_ENTERS, add_four, "When this creature enters, if it was bargained, add four mana in any combination of colors.")
            .condition(bargain::was_bargained),
        ability(cost("{1}{R}"), effects::damage_first_target::<1>, "{1}{R}: This creature deals 1 damage to any target.").targets(&[targets::ANY]),
    ],
    ..oracle::REALM_SCORCHER_HELLKITE
};
