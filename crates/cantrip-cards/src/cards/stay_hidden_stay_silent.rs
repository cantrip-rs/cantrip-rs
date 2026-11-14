use cantrip_api::*;
use cantrip_mechanics::{
    attach::{attached_to_it, tap_enchanted},
    manifest_dread, targets,
    triggers::THIS_ENTERS,
};
use cantrip_oracle as oracle;

/// "Shuffle enchanted creature into its owner's library, then manifest dread." (CR 701.24, 701.62a): the creature this Aura
/// is attached to as the ability resolves (CR 303.4m, 608.2h); it is shuffled into its owner's library, which is shuffled even
/// if the creature can't get there (CR 701.24c), and then the ability's controller manifests dread from their own library.
fn shuffle_enchanted_then_manifest_dread(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = g.obj(item.object).attached_to {
        g.put_into_library(&[creature], LibraryPlace::ShuffledIn);
    }
    manifest_dread::controller_manifests_dread(g, item)
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[
        triggered(THIS_ENTERS, tap_enchanted, "When this Aura enters, tap enchanted creature."),
        ability(
            cost("{4}{U}{U}"),
            shuffle_enchanted_then_manifest_dread,
            "{4}{U}{U}: Shuffle enchanted creature into its owner's library, then manifest dread. Activate only as a sorcery.",
        )
        .sorcery_speed(),
    ],
    // "Enchanted creature doesn't untap during its controller's untap step" (CR 502.3): a rule of the Aura's, not an ability
    // of the creature (CR 113.1a).
    untap_bans: &[attached_to_it],
    ..oracle::STAY_HIDDEN_STAY_SILENT
};
