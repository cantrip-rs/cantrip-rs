use cantrip_api::*;
use cantrip_mechanics::replacements::{plus_one_counters_applies, plus_one_counters_apply_to_proposal};
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::cycling;
use cantrip_oracle as oracle;

/// "If one or more +1/+1 counters would be put on an artifact or creature you control, that many plus one +1/+1
/// counters are put on it instead." (CR 122.6, 614.1a.)
fn applies(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    plus_one_counters_applies(
        q,
        asker,
        proposal,
        CounterKind::PlusOne,
        &[CardType::Artifact, CardType::Creature],
    )
}

fn put_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(permanent) = item.target(0) {
        g.add_counters(permanent, CounterKind::PlusOne, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{1}{G}, {T}"), put_counter, "{1}{G}, {T}: Put a +1/+1 counter on target artifact or creature you control. Activate only as a sorcery.")
            .sorcery_speed()
            .targets(&[targets::ARTIFACT_OR_CREATURE_YOU_CONTROL]),
        cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)"),
    ],
    replacements: &[replacement(ReplacementKind::Counters, applies, plus_one_counters_apply_to_proposal, "puts one more +1/+1 counter")],
    ..oracle::OZOLITH_THE_SHATTERED_SPIRE
};
