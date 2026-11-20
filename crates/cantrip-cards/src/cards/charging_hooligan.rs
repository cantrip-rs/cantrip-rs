use cantrip_api::*;
use cantrip_mechanics::{attacking_creatures, effects, pump, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks, it gets +1/+0 until end of turn for each attacking creature. If a Rat is attacking, this
/// creature gains trample until end of turn." (CR 508.1k, 611.2a, 608.2c): the attackers are counted as the ability resolves,
/// whoever controls them and this creature included, and the Rat is asked once the pump is done, among every attacking creature
/// (this creature too, were it a Rat).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let attackers = attacking_creatures(g) as i32;
    pump(g, item.object, item.object, attackers, 0);
    g.end_sentence();
    let rat_word = g.words(item.object).subtype(Subtype::Rat);
    let rat = g.state().battlefield().iter().any(|&id| g.is_attacking(id) && g.chars(id).has_subtype(rat_word));
    if rat {
        effects::this_gains_until_end_of_turn(g, item, Keyword::Trample)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        effect,
        "Whenever this creature attacks, it gets +1/+0 until end of turn for each attacking creature. If a Rat is attacking, this creature gains trample until end of turn.",
    )],
    ..oracle::CHARGING_HOOLIGAN
};
