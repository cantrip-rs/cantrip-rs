use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "When this artifact enters, gain control of target creature until end of turn. Untap that creature. It gains haste until end
/// of turn." (CR 613.1b, 611.2a, 702.10): the shared Act of Treason steps for the trigger's target, nothing at all if it is no
/// longer legal (CR 608.2b).
fn gain_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_control_untap_haste(g, item);
    Ok(())
}

/// A Food artifact (CR 111.10b names the Food ability a Food token has; this card prints its own): two activated abilities that
/// share the cost "{2}, {T}, Sacrifice this artifact", each its own ability, the second with a target opponent.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            gain_control,
            "When this artifact enters, gain control of target creature until end of turn. Untap that creature. It gains haste until end of turn.",
        )
        .targets(&[targets::CREATURE]),
        ability(cost("{2}, {T}").and(SACRIFICE_THIS), effects::gain_three_life, "{2}, {T}, Sacrifice this artifact: You gain 3 life."),
        ability(
            cost("{2}, {T}").and(SACRIFICE_THIS),
            effects::target_player_loses_life::<3>,
            "{2}, {T}, Sacrifice this artifact: Target opponent loses 3 life.",
        )
        .targets(&[targets::OPPONENT]),
    ],
    ..oracle::ERIETTES_TEMPTING_APPLE
};
