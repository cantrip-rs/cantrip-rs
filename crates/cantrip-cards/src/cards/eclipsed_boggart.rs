use cantrip_api::*;
use cantrip_mechanics::library::look_for_type_or_two_basic_land_types;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this creature enters, look at the top four cards of your library. You may reveal a Goblin, Swamp, or Mountain card from among them and put it into your hand. Put the rest on the bottom of your library in a random order." (CR 401.4, 701.20a)
fn look(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    look_for_type_or_two_basic_land_types(g, item, Subtype::Goblin, Subtype::Swamp, Subtype::Mountain)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, look, "When this creature enters, look at the top four cards of your library. You may reveal a Goblin, Swamp, or Mountain card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.")],
    ..oracle::ECLIPSED_BOGGART
};
