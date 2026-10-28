use cantrip_api::*;
use cantrip_mechanics::amass::amass;
use cantrip_mechanics::{statics, tokens};
use cantrip_oracle as oracle;

/// "Amass Goblins 2." (CR 701.47a.)
fn amass_goblins(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    amass(g, item.controller, item.object, &tokens::GOBLIN_ARMY, Subtype::Goblin, 2)
}

/// The Adventure (CR 715): Clap! Snap!
pub static CLAP_SNAP: CardDef = CardDef { spell: spell(&[], amass_goblins), ..oracle::CLAP_SNAP };

/// "Each creature you control with a +1/+1 counter on it has menace" (CR 613.1f, 702.111).
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        statics::your_creatures_with_plus_one_counters,
        statics::have_menace,
        "Each creature you control with a +1/+1 counter on it has menace.",
    )],
    adventure: Some(&CLAP_SNAP),
    ..oracle::GREAT_UGLY_LOOKING_GOBLIN
};

pub static EXTRA: &[&CardDef] = &[&CLAP_SNAP, &tokens::GOBLIN_ARMY];
