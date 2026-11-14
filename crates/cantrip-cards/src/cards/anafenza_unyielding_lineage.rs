use cantrip_api::*;
use cantrip_mechanics::{endure, triggers};
use cantrip_oracle as oracle;

/// "Whenever another nontoken creature you control dies, Anafenza endures 2." (CR 701.63a, 603.10a): Anafenza endures even when
/// it died in the same event, and then only the Spirit token can be made.
fn endures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    endure::endure(g, item.object, item.object, 2)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::ANOTHER_NONTOKEN_CREATURE_YOU_CONTROL_DIES,
        endures,
        "Whenever another nontoken creature you control dies, Anafenza endures 2. (Put two +1/+1 counters on it or create a 2/2 white Spirit creature token.)",
    )],
    ..oracle::ANAFENZA_UNYIELDING_LINEAGE
};

pub static EXTRA: &[&CardDef] = &[&endure::SPIRIT];
