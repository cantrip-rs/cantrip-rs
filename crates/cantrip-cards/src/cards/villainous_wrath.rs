use cantrip_api::*;
use cantrip_mechanics::{creatures_you_control, effects, targets};
use cantrip_oracle as oracle;

/// "Target opponent loses life equal to the number of creatures they control. Then destroy all creatures." (CR 119.3, 701.8a): the
/// creatures are counted as the spell resolves, before any is destroyed; a spell whose only target has become illegal does nothing
/// at all (CR 608.2b), the creatures staying.
fn drain_then_wrath(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let creatures = creatures_you_control(g, opponent);
    g.lose_life(opponent,(creatures) as i32);
    g.end_sentence();
    effects::destroy_all_creatures(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::OPPONENT], drain_then_wrath), ..oracle::VILLAINOUS_WRATH };
