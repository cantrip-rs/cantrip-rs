use cantrip_api::*;
use cantrip_mechanics::cumulative_upkeep;
use cantrip_mechanics::cumulative_upkeep::Payment;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[cumulative_upkeep!(Payment::Sacrifice(targets::filters::creature), "Cumulative upkeep—Sacrifice a creature. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)")],
    ..oracle::PHYREXIAN_SOULGORGER
};
