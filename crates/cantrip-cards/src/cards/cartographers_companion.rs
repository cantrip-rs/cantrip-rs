use cantrip_api::*;
use cantrip_mechanics::tokens::create_map;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "When this creature enters, create a Map token." (CR 111.10s)
fn map(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    create_map(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, map, "When this creature enters, create a Map token.")],
    ..oracle::CARTOGRAPHERS_COMPANION
};
