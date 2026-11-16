use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, lands, statics, targets};
use cantrip_oracle as oracle;

/// "Choose target creature. If that creature's power is 2 or less or if you control another Lizard, gain control of that creature
/// until end of turn, untap it, and it gains haste until end of turn." (CR 611.2a, 613.1b, 608.2b, 702.10): the target is any creature,
/// and the condition is judged as the ability resolves (ruling 2024-07-26) - its power as the layers give it now, and another Lizard being
/// a permanent of the controller's other than this one, read through the words of this card's text (CR 612); if this creature has left the
/// battlefield by then, any Lizard they control is another one (CR 400.7). With neither true the ability does nothing; with the target
/// illegal it does nothing at all.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let another_lizard = statics::you_control_another::<{ lands::sub(Subtype::Lizard) }>(g, &g.item_asker(item, item.controller));
    if another_lizard || g.chars(creature).power.is_some_and(|power| power <= 2) {
        effects::gain_control_untap_haste(g, item);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effect, "When this creature enters, choose target creature. If that creature's power is 2 or less or if you control another Lizard, gain control of that creature until end of turn, untap it, and it gains haste until end of turn.")
        .targets(&[targets::CREATURE])],
    ..oracle::REPTILIAN_RECRUITER
};
