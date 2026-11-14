use cantrip_api::*;
use cantrip_mechanics::{manifest_dread::manifest_dread, targets};
use cantrip_oracle as oracle;

/// "Destroy target creature. Its controller manifests dread" (CR 701.8, 701.62a, 608.2h): the target's controller is read before the
/// destroy, and manifests dread whether or not it was destroyed (indestructible, regenerated); an illegal target does nothing at all
/// (CR 608.2b).
fn destroy_then_its_controller_manifests_dread(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let controller = g.controller(creature);
    g.destroy(creature);
    g.end_sentence();
    manifest_dread(g, controller, item.object)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], destroy_then_its_controller_manifests_dread),
    ..oracle::UNWANTED_REMAKE
};
