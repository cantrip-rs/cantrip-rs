use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

const TEXT: &str = "As long as you control an artifact, this creature gets +1/+0 and has deathtouch.";

/// "As long as you control an artifact" (CR 604.1, 613.1f): both of the creature's effects ask `statics::itself_while_you_control_an_artifact`.
pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::Ability, statics::itself_while_you_control_an_artifact, statics::have_deathtouch, TEXT),
        static_ability(Layer::PtModify, statics::itself_while_you_control_an_artifact, statics::plus_pt::<1, 0>, TEXT),
    ],
    ..oracle::GRAVBLADE_HEAVY
};
