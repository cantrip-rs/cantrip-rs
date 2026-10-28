use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Gain control of target creature until end of turn. If you control a Villain with greater mana value than that creature, gain
/// control of that creature until the end of your next turn instead. Untap that creature. It gains haste until end of turn."
/// (CR 611.2a, 613.1b, 608.2b, 202.3): the Villain comparison is made as the spell resolves, with the target's mana value as it is
/// then (a Villain read through the words of this card's text, CR 612); a Villain of equal or lesser value is not enough. The control
/// is one effect, of one duration - "instead" replaces the shorter one, it does not add to it - then the creature untaps and gains
/// haste, as `gain_control_untap_haste` does for Act of Treason. A spell whose target has become illegal does nothing (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let villain = g.words(item.object).subtype(Subtype::Villain);
    let value = g.mana_value(creature);
    let greater_villain = g.permanents_of(item.controller).into_iter().any(|id| g.chars(id).has_subtype(villain) && g.mana_value(id) > value);
    let duration = if greater_villain { Duration::EndOfNextTurnOf(item.controller) } else { Duration::EndOfTurn };
    effects::gain_control_untap_haste_for(g, item, duration);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::EVILS_THRALL };
