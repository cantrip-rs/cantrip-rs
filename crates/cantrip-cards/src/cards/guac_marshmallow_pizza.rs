use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +2/+2 until end of turn. Untap it." (CR 611.2a, 701.26): two sentences, so the untap is its own
/// instruction.
fn pump_and_untap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_creature_gets::<2, 2>(g, item)?;
    g.end_sentence();
    effects::untap_targets(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, pump_and_untap, "When this artifact enters, target creature gets +2/+2 until end of turn. Untap it.")
            .targets(&[targets::CREATURE]),
        ability(cost("{2}, {T}").and(SACRIFICE_THIS), effects::gain_three_life, "{2}, {T}, Sacrifice this artifact: You gain 3 life."),
    ],
    ..oracle::GUAC_MARSHMALLOW_PIZZA
};
