use cantrip_api::*;
use cantrip_mechanics::{effects, first_target_player_sacrifices_a_creature, targets};
use cantrip_oracle as oracle;

/// "Target opponent sacrifices a creature of their choice. You gain life equal to that creature's toughness." (CR 701.21a, 608.2h): the toughness is the sacrificed creature's as it last
/// existed on the battlefield. An opponent who sacrificed nothing (no creature, or an effect stopping it) leaves no creature to read, so nothing is gained.
fn sacrifice_and_gain_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let left = first_target_player_sacrifices_a_creature(g, item)?;
    let Some(&creature) = left.permanents().first() else { return Ok(()) };
    effects::gain_life_equal_to_toughness_of(g, item.controller, creature)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::OPPONENT], sacrifice_and_gain_life), ..oracle::TRIBUTE_TO_HUNGER };
