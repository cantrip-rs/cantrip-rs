use cantrip_api::*;
use cantrip_mechanics::{damage_divided, targets};
use cantrip_oracle as oracle;

/// "Fiery Justice deals 5 damage divided as you choose among any number of targets.
/// Target opponent gains 5 life." (CR 608.2b, 608.2c, 119.3).
fn fiery_justice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponent = g.state().spec_targets(item, 1).next().and_then(|slot| targets::player(g, item, slot));
    damage_divided(g, item);
    g.end_sentence();
    if let Some(opponent) = opponent {
        g.gain_life(opponent, 5);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ANY.between(0, 5), targets::OPPONENT], fiery_justice),
    divide: Some(Divide { total: 5, reads: None }),
    ..oracle::FIERY_JUSTICE
};
