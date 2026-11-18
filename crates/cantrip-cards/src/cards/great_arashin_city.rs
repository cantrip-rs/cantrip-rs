use cantrip_api::*;
use cantrip_mechanics::{endure, tokens};
use cantrip_mechanics::lands::{enters_tapped_unless_you_control_either, sub};
use cantrip_mechanics::targets::filters;
use cantrip_oracle as oracle;

/// "{1}{B}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token." The exiled card is
/// part of the cost (CR 602.2b, 601.2h): with no creature card in the graveyard the cost can't be paid, so the ability can't be
/// activated at all (CR 118.3). The token is endure's white Spirit, made 1/1 (CR 111.3).
fn create_a_spirit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_sized(g, item.object, item.controller, &endure::SPIRIT, (1, 1), 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_tapped_unless_you_control_either::<{ sub(Subtype::Forest) }, { sub(Subtype::Plains) }>),
    abilities: &[
        mana_ability(TAP, add(Mana::B), "{T}: Add {B}."),
        ability(
            cost("{1}{B}, {T}").and(exile_from_graveyard_of(1, filters::creature)),
            create_a_spirit,
            "{1}{B}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token.",
        ),
    ],
    ..oracle::GREAT_ARASHIN_CITY
};

pub static EXTRA: &[&CardDef] = &[&endure::SPIRIT];
