use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +2/+2 and gains first strike until end of turn. Untap it." (CR 611.2a, 702.7, 701.26a.)
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_creature_gets_and_gains_first_strike::<2, 2>(g, item)?;
    g.end_sentence();
    effects::untap_targets(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::RIVERGUARDS_REFLEXES };
