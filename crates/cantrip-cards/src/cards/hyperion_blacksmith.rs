use cantrip_api::*;
use cantrip_mechanics::{choose, targets};
use cantrip_oracle as oracle;

/// "{T}: You may tap or untap target artifact an opponent controls." (CR 608.2, 701.26a, 701.26b).
fn tap_untap_target_artifact_opponent(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let card = g.card_id(&CARD);
    choose::may_tap_or_untap_target(g, item, card)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                tap_untap_target_artifact_opponent,
                "{T}: You may tap or untap target artifact an opponent controls.",
            )
            .targets(&[targets::ARTIFACT_AN_OPPONENT_CONTROLS]),
    ],
    labels: choose::TAP_OR_UNTAP,
    ..oracle::HYPERION_BLACKSMITH
};
