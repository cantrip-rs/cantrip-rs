use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Remove two counters from this creature" (CR 118.3): counters of any kinds, taken from the source alone.
fn this_creature(_: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    id == asker.source
}

/// "Return target creature card with mana value 3 or less from your graveyard to the battlefield." Under the
/// ability's controller's control (CR 400.3, 110.2).
fn reanimate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    g.put_onto_battlefield(card, item.controller, false)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_minus_one_counters::<2>),
    abilities: &[ability(
        cost("{1}{W/B}").and(remove_counters_among(2, this_creature)),
        reanimate,
        "{1}{W/B}, Remove two counters from this creature: Return target creature card with mana value 3 or less from your graveyard to the battlefield. Activate only as a sorcery.",
    )
    .targets(&[targets::CREATURE_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD])
    .sorcery_speed()],
    ..oracle::REAPING_WILLOW
};
