use cantrip_api::*;
use cantrip_mechanics::{targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Tap up to two target creatures": each target still legal is tapped (CR 608.2b).
fn tap_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for creature in targets::legal_objects(g, item) {
        g.tap(creature);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[cantrip_mechanics::cost_reduction::for_each_instant_and_sorcery_in_your_graveyard],
    enters: Some(cantrip_mechanics::mana::enters_tapped_unless_your_turn),
    abilities: &[triggered(THIS_ENTERS, tap_them, "When this creature enters, tap up to two target creatures.")
        .targets(&[targets::CREATURE.between(0, 2)])],
    ..oracle::EDDYMURK_CRAB
};
