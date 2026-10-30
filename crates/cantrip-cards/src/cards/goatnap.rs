use cantrip_api::*;
use cantrip_mechanics::{effects::gain_control_untap_haste, pump, targets};
use cantrip_oracle as oracle;

/// "Gain control of target creature until end of turn. Untap that creature. It gains haste until end of turn. If that creature
/// is a Goat, it also gets +3/+0 until end of turn." (CR 613.1b, 611.2a, 702.10, 608.2b): Act of Treason's three sentences, then
/// the Goat check (read through the words of this card's text, CR 612), made as the spell resolves, on the creature as it is then; nothing if the target is no longer legal.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let goat = g.words(item.object).subtype(Subtype::Goat);
    let creature = gain_control_untap_haste(g, item);
    g.end_sentence();
    if let Some(creature) = creature
        && g.chars(creature).has_subtype(goat)
    {
        pump(g, item.object, creature, 3, 0);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::GOATNAP };
