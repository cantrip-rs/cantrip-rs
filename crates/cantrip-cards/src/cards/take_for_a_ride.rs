use cantrip_api::*;
use cantrip_mechanics::{effects::gain_control_untap_haste, history::committed_a_crime_this_turn, targets};
use cantrip_oracle as oracle;

/// "Take for a Ride has flash as long as you've committed a crime this turn" (CR 702.8a, 700.13): asked as the casting begins,
/// from whatever zone it is cast, so it is castable at instant speed on any turn its controller has targeted an opponent or something
/// of theirs, and only on that turn.
fn flash_after_a_crime(q: &dyn Query, asker: &Asker) -> bool {
    committed_a_crime_this_turn(q.history(), asker.controller)
}

/// "Gain control of target creature until end of turn. Untap that creature. It gains haste until end of turn." (CR 613.1b,
/// 611.2a, 702.10): any creature, one its caster already controls too (the card's ruling); nothing if the target is no longer
/// legal (CR 608.2b). Its Auras and Equipment do not come with it (the ruling).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    gain_control_untap_haste(g, item);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), flash_if: Some(flash_after_a_crime), ..oracle::TAKE_FOR_A_RIDE };
