use cantrip_api::*;
use cantrip_mechanics::{effects, permanents_with_subtype_you_control};
use cantrip_oracle as oracle;

/// "Creatures you control get +2/+1 until end of turn. If you control a Rabbit, scry 2." (CR 611.2c, 608.2c, 701.22): the pump
/// first, then, as a second instruction, whether the caster controls a Rabbit as it is followed - read through the layers (a
/// changeling counts) and by the word the spell's text says now (CR 612.2) - before the scry.
fn rabbit_response(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::pump_creatures_you_control(g, item, 2, 1);
    g.end_sentence();
    let rabbit = g.words(item.object).subtype(Subtype::Rabbit);
    if permanents_with_subtype_you_control(g, item.controller, rabbit) > 0 {
        effects::scry_two(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], rabbit_response), ..oracle::RABBIT_RESPONSE };
