use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(effects::tap_targets, "Tap target creature.").targets(&[targets::CREATURE]),
    mode(effects::untap_targets, "Untap target creature.").targets(&[targets::CREATURE]),
    mode(|g, item| cantrip_mechanics::effects::loot(g, item), "Draw a card, then discard a card."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(THIS_ENTERS, &MODAL, "When this creature enters, choose one \u{2014} Tap target creature; or untap target creature; or draw a card, then discard a card.")],
    ..oracle::DIVINING_DUELIST
};
