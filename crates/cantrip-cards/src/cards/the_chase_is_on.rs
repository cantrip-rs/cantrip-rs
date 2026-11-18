use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Target creature gets +3/+0 and gains first strike until end of turn. Investigate." (CR 611.2a, 701.16a): the pump and the Clue
/// are two sentences, so two instructions. If the target is no longer legal as the spell resolves it does not resolve at all
/// (CR 608.2b) and no Clue is made (its ruling).
fn chase(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_creature_gets_and_gains_first_strike::<3, 0>(g, item)?;
    g.end_sentence();
    tokens::investigate(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], chase),
    ..oracle::THE_CHASE_IS_ON
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
