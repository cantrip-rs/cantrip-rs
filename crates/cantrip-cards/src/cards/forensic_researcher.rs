use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{T}, Collect evidence 3" (CR 701.59a, 602.2): tapping it and exiling cards with total mana value 3 or more from the graveyard
/// are paid together as the ability is activated, so with too few cards (CR 701.59b) it cannot be activated at all.
const TAP_AND_COLLECT_EVIDENCE_3: Cost = Cost { tap: true, collect_evidence: 3, ..Cost::NONE };

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, effects::untap_targets, "{T}: Untap another target permanent you control.").targets(&[targets::ANOTHER_PERMANENT_YOU_CONTROL]),
        ability(TAP_AND_COLLECT_EVIDENCE_3, effects::tap_targets, "{T}, Collect evidence 3: Tap target creature you don't control. (To collect evidence 3, exile cards with total mana value 3 or greater from your graveyard.)")
            .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
    ],
    ..oracle::FORENSIC_RESEARCHER
};
