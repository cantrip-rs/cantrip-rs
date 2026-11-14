use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "~ deals 6 damage to target creature. You may discard a card. If you do, draw a card." (CR 120.3, 701.9a, 608.2b): the
/// creature is the only target, so when it is no longer legal the spell does not resolve and nothing is discarded or drawn (the
/// ruling). The discard and draw happen whether or not the damage killed it, and the draw only follows a card actually discarded.
fn incinerating_blast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_first_target::<6>(g, item)?;
    g.end_sentence();
    effects::may_discard_to_draw(g, item, 1)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], incinerating_blast), ..oracle::INCINERATING_BLAST };
