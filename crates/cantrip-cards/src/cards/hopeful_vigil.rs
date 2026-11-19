use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_DIES, THIS_ENTERS};
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Create a 2/2 white Knight creature token with vigilance" (CR 111.2, 702.20).
fn create_a_knight(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::KNIGHT, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, create_a_knight, "When this enchantment enters, create a 2/2 white Knight creature token with vigilance."),
        // Sacrificed or destroyed, it is looked back on (CR 603.10a) and scries from the graveyard.
        triggered(THIS_DIES, effects::scry_two, "When this enchantment is put into a graveyard from the battlefield, scry 2."),
        ability(cost("{2}{W}"), effects::sacrifice_self, "{2}{W}: Sacrifice this enchantment."),
    ],
    ..oracle::HOPEFUL_VIGIL
};

pub static EXTRA: &[&CardDef] = &[&tokens::KNIGHT];
