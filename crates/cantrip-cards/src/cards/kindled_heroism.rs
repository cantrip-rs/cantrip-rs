use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +1/+0 and gains first strike until end of turn. Scry 1." The pump is one sentence's continuous effects
/// (CR 611.2a). A spell whose only target has become illegal does not resolve at all (CR 608.2b), so the scry never happens then.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_creature_gets_and_gains_first_strike::<1, 0>(g, item)?;
    g.end_sentence();
    effects::scry(g, item, 1)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::KINDLED_HEROISM };
