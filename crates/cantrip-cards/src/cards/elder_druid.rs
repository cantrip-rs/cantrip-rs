use cantrip_api::*;
use cantrip_mechanics::{choose, targets};
use cantrip_oracle as oracle;

/// "{3}{G}, {T}: You may tap or untap target artifact, creature, or land." (CR 608.2, 701.26a, 701.26b).
fn tap_untap_target_artifact_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let card = g.card_id(&CARD);
    choose::may_tap_or_untap_target(g, item, card)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{3}{G}").and(TAP),
                tap_untap_target_artifact_creature,
                "{3}{G}, {T}: You may tap or untap target artifact, creature, or land.",
            )
            .targets(&[targets::ARTIFACT_CREATURE_OR_LAND]),
    ],
    labels: choose::TAP_OR_UNTAP,
    ..oracle::ELDER_DRUID
};
