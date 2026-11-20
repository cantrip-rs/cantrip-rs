use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { spell: spell(&[], library::look_for_a_creature_rest_on_the_bottom_in_any_order::<5>), ..oracle::COMMUNE_WITH_NATURE };
