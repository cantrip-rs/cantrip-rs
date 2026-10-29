use cantrip_api::*;
use cantrip_mechanics::statics::{itself_while_you_gained_life, plus_pt};
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("Draw", "draw a card?", "", "Draw", "Don't draw");

/// "You may draw a card" (CR 121.2b): the trigger resolves before the spell or ability that caused it, and happens once however
/// many times the creature was targeted at once.
fn may_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_draw(g, item.controller, item.object, 1, &ASK)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_BECOMES_THE_TARGET_OF_AN_OPPONENTS_SPELL_OR_ABILITY,
        may_draw,
        "Whenever this creature becomes the target of a spell or ability an opponent controls, you may draw a card.",
    )],
    statics: &[static_ability(
        Layer::PtModify,
        itself_while_you_gained_life,
        plus_pt::<2, 0>,
        "Infusion \u{2014} This creature gets +2/+0 as long as you gained life this turn.",
    )],
    ..oracle::TENURED_CONCOCTER
};
