use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gets -6/-6 until end of turn. You gain 2 life." (CR 611.2a, 119.3, 608.2b): both are lost if the target is no
/// longer legal as the spell resolves, since a spell with only illegal targets doesn't resolve.
fn shrink_and_gain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_creature_gets::<-6, -6>(g, item)?;
    g.end_sentence();
    effects::gain_life(g, item, 2)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], shrink_and_gain), ..oracle::SYPHON_FUEL };
