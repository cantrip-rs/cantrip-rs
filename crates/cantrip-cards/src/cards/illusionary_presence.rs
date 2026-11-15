use cantrip_api::*;
use cantrip_mechanics::cumulative_upkeep::Payment;
use cantrip_mechanics::{cumulative_upkeep, effects, text_change, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, choose a land type. This creature gains landwalk of the chosen type until end of
/// turn." The land type is chosen as the ability resolves (CR 608.2d), by its controller, from every land type (CR 205.3i):
/// the 2005-11-01 rulings allow any land subtype that exists and no snow landwalk, snow being a supertype. The creature
/// gains the landwalk naming it (CR 702.14c: it can't be blocked as long as the defending player controls a land of that
/// type), a layer 6 effect for the rest of the turn (CR 611.2a, 613.1f); nothing if it is no longer the object it was
/// (CR 400.7).
fn presence(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let chosen = text_change::one_subtype(g, item.controller, item.object, SubtypeSet::LAND_TYPES.iter().collect(), WordUse::Chosen)?;
    // Every land type offered has its landwalk.
    let Some(walk) = Keyword::landwalk_of(chosen) else { return Ok(()) };
    effects::this_gains_until_end_of_turn(g, item, walk)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(Payment::Mana(mana("{U}")), "Cumulative upkeep {U} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)"),
        triggered(triggers::YOUR_UPKEEP, presence,
            "At the beginning of your upkeep, choose a land type. This creature gains landwalk of the chosen type until end of turn."),
    ],
    ..oracle::ILLUSIONARY_PRESENCE
};
