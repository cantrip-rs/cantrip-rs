use cantrip_api::*;
use cantrip_mechanics::{effects, surveil, targets};
use cantrip_oracle as oracle;

/// "Target opponent discards two cards. Then if you control a Rat, surveil 2." (CR 701.9a, 701.25a): the opponent chooses the
/// discards; the Rat is looked for after they are done, as the sentence says "then" (a Rat read through the words of the card's
/// text, CR 612, under this player's control). A spell whose only target is illegal never resolves (CR 608.2b), so there is no
/// surveil without the discard.
fn target_opponent_discards_two_then_surveil_2_if_you_control_a_rat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_player_discards::<2>(g, item)?;
    let rat = g.words(item.object).subtype(Subtype::Rat);
    if g.permanents_of(item.controller).into_iter().any(|id| g.chars(id).has_subtype(rat)) {
        surveil(g, item.controller, item.object, 2)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::OPPONENT], target_opponent_discards_two_then_surveil_2_if_you_control_a_rat),
    ..oracle::PSYCHIC_WHORL
};
