use cantrip_api::*;
use cantrip_mechanics::{effects, manifest_dread, targets};
use cantrip_oracle as oracle;

/// "Return up to one target nonland permanent to its owner's hand. Manifest dread." (CR 400.3, 701.62a): the manifest is a
/// new sentence (CR 608.2c) and is done whether or not a target was chosen; a spell whose only target has become illegal does
/// nothing at all (CR 608.2b), which the engine decides before this runs.
fn return_then_manifest_dread(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_first_target_to_hand(g, item)?;
    manifest_dread::controller_manifests_dread(g, item)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::NONLAND_PERMANENT.optional()], return_then_manifest_dread),
    ..oracle::UNNERVING_GRASP
};
