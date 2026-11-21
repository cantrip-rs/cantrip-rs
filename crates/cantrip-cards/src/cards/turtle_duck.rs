use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Until end of turn, this creature has base power 4 and gains trample." (CR 611.2a, 613.4b): one instruction of a layer 7b effect that
/// sets only the power (the toughness, counters and pumps are left alone, CR 613.4c) and a layer 6 keyword grant, both from this
/// creature for the rest of the turn.
fn base_power_four_and_trample(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        g.add_effect(item.object, item.object, EffectKind::SetPower(4), Duration::EndOfTurn);
        effects::gain_until_end_of_turn(g, item.object, item.object, Keyword::Trample);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{3}"), base_power_four_and_trample, "{3}: Until end of turn, this creature has base power 4 and gains trample.")],
    ..oracle::TURTLE_DUCK
};
