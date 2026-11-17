use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Creatures target opponent controls get -1/-1 until end of turn. Surveil 2." (CR 611.2c, 701.25): the creatures that opponent
/// controls as it resolves shrink, then the caster surveils. A spell whose only target has become illegal does not resolve at
/// all (CR 608.2b, the core's), so there is no surveil then.
fn shrink_their_creatures_then_surveil(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::creatures_target_player_controls_get::<0, -1, -1>(g, item)?;
    g.end_sentence();
    effects::surveil(g, item, 2)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::OPPONENT], shrink_their_creatures_then_surveil), ..oracle::NEUTRALIZE_THE_GUARDS };
