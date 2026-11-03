use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::{protection, triggers};
use cantrip_oracle as oracle;

const GAIN: Question = Question::new("GainLife", "gain 1 life?", "A red spell was cast", "Gain 1 life", "Don't gain life");

/// "... you may gain 1 life." (CR 119.3): asked of this ability's controller as it resolves.
fn may_gain_one_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if ask_may(g, item.controller, item.object, Purpose::Ask(&GAIN))? {
        g.gain_life(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[protection::from::<{ Color::Red as u8 }>("Protection from red")],
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, triggers::a_player_casts_colored_spell::<{Color::Red as u8}>),
        may_gain_one_life,
        "Whenever a player casts a red spell, you may gain 1 life.",
    )],
    ..oracle::KOR_FIREWALKER
};
