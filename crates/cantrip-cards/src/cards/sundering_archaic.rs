use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{library, spent, targets};
use cantrip_oracle as oracle;

/// "Target nonland permanent an opponent controls with mana value less than or equal to the number of colors of mana spent to
/// cast this creature" (CR 115.1, 202.3): the colors are those spent to cast the source, read from the source itself (CR 601.2h),
/// both as the trigger is put on the stack and again as it resolves (CR 608.2b). Colorless is not a color (CR 105.1).
fn nonland_permanent_an_opponent_controls_that_costs_no_more_than_the_colors_spent(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::NONLAND_PERMANENT.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if q.controller(id) != asker.controller
            && q.mana_value(id) <= spent::colors_of_mana_spent(q, asker.source))
}

/// "Exile target nonland permanent ...": the target is exiled as the ability resolves, if it is still a legal target.
fn exile_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    g.exile(permanent);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            exile_target,
            "Converge \u{2014} When this creature enters, exile target nonland permanent an opponent controls with mana value less than or equal to the number of colors of mana spent to cast this creature.",
        )
        .targets(&[target(nonland_permanent_an_opponent_controls_that_costs_no_more_than_the_colors_spent)]),
        ability(cost("{2}"), library::put_first_target_on_the_bottom, "{2}: Put target card from a graveyard on the bottom of its owner's library.")
            .targets(&[targets::CARD_IN_A_GRAVEYARD]),
    ],
    ..oracle::SUNDERING_ARCHAIC
};
