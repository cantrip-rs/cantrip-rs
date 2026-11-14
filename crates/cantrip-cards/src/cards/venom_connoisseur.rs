use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// This creature gains deathtouch until end of turn; if this is the second time this ability has resolved this turn,
/// all creatures you control gain it too (CR 608.2c, 611.2c). Earlier resolutions are this source's, counted in the
/// turn's history.
fn alliance(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::this_gains_deathtouch(g, item)?;
    g.end_sentence();
    if g.history().resolutions(item.object, item.ability) == 1 {
        effects::creatures_you_control_gain(g, item, &[Keyword::Deathtouch]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
        alliance,
        "Alliance \u{2014} Whenever another creature you control enters, this creature gains deathtouch until end of turn. If this is the second time this ability has resolved this turn, all creatures you control gain deathtouch until end of turn.",
    )],
    ..oracle::VENOM_CONNOISSEUR
};
