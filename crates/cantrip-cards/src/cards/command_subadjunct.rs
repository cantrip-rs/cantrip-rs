use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // Vigilance is a static ability in layer 6; attacking with a vigilant creature doesn't tap it (CR 613.1f, 702.20a, 702.20b).
    statics: &[static_ability(
        Layer::Ability,
        statics::your_artifact_creatures,
        statics::have_vigilance,
        "Artifact creatures you control have vigilance.",
    )],
    ..oracle::COMMAND_SUBADJUNCT
};
