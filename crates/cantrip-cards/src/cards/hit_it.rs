use cantrip_api::*;
use cantrip_mechanics::{library, statics, triggers};
use cantrip_oracle as oracle;

/// "Reveal the top five cards ... Put a land card ... tapped and the rest into your graveyard." (CR 701.20, 608.2c).
fn reveal_five(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::reveal_top_land_to_battlefield_rest_to_graveyard(g, item.controller, item.object, 5, true)
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, statics::your_creatures, statics::have_haste, "Creatures you control have haste.")],
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        reveal_five,
        "When this enchantment enters, reveal the top five cards of your library. Put a land card from among them onto the battlefield tapped and the rest into your graveyard.",
    )],
    ..oracle::HIT_IT
};
