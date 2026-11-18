use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::statics::{have_cant_be_blocked, itself_with_permanent_cards_in_your_graveyard};
use cantrip_mechanics::craft;
use cantrip_oracle as oracle;

/// "{T}: Mill a card." (CR 701.17)
fn mill_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.mill(item.controller, 1);
    Ok(())
}

/// "Craft with Island": an Island you control or an Island card from your graveyard (CR 702.167b), read through the Hulk's words (CR 612).
static ISLAND: Materials = craft::materials(1, false, cantrip_mechanics::targets::filters::land_type::<{ cantrip_mechanics::lands::sub(Subtype::Island) }>, "Island");

/// "Descend 8 - This Vehicle can't be blocked as long as there are eight or more permanent cards in your graveyard" (CR
/// 207.2c: descend is an ability word; CR 110.4b): counted in its controller's graveyard whenever its characteristics are.
pub static BACK: CardDef = CardDef {
    abilities: &[crew(1, "Crew 1")],
    statics: &[static_ability(Layer::Ability, itself_with_permanent_cards_in_your_graveyard::<8>, have_cant_be_blocked, "Descend 8 \u{2014} This Vehicle can't be blocked as long as there are eight or more permanent cards in your graveyard.")],
    ..oracle::WATERTIGHT_GONDOLA
};

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, mill_one, "{T}: Mill a card."), craft::ability(mana("{3}{U}"), &ISLAND, "Craft with Island {3}{U}")],
    back: Some(&BACK),
    ..oracle::WATERLOGGED_HULK
};
