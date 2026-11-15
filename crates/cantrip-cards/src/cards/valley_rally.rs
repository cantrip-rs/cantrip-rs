use cantrip_api::*;
use cantrip_mechanics::{effects, gift, targets, tokens};
use cantrip_oracle as oracle;

/// "Creatures you control get +2/+0 until end of turn. If the gift was promised, target creature you control gains first strike until
/// end of turn." (CR 702.174, 611.2, 608.2b): the gift was given before this runs (CR 702.174j). The creatures are those you control as
/// the spell resolves, and the pump is one sentence, the first strike another (CR 608.2c). The creature is a target only when the gift was
/// promised (CR 601.2c, `TargetSpec::if_promised`), so without a promise there is none to give first strike; a target no longer legal
/// gets nothing, while the pump still happens.
fn rally(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::pump_creatures_you_control(g, item, 2, 0);
    // A sentence ends (CR 608.2c): the state triggers judge what it did before the next (`Game::end_sentence`).
    g.end_sentence();
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_until_end_of_turn(g, item.object, creature, Keyword::FirstStrike);
    Ok(())
}

/// "Gift a Food" (CR 702.174d): the Food is a token of its own.
pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL.between(0, 0).if_promised(1, 1)], rally),
    gift: Some(gift::FOOD),
    ..oracle::VALLEY_RALLY
};
