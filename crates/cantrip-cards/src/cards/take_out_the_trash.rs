use cantrip_api::*;
use cantrip_mechanics::{damage_target, effects, permanents_with_subtype_you_control, targets};
use cantrip_oracle as oracle;

/// "Take Out the Trash deals 3 damage to target creature or planeswalker. If you control a Raccoon, you may discard a card. If you do,
/// draw a card." (CR 608.2c, 701.9): the loot follows the damage, and whether you control a Raccoon is judged as it resolves, through
/// the words of this card's text (CR 612) and the layers, so a changeling counts; a spell whose only target became illegal does
/// nothing at all (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 3);
    let raccoon = g.words(item.object).subtype(Subtype::Raccoon);
    if permanents_with_subtype_you_control(g, item.controller, raccoon) > 0 {
        effects::may_discard_to_draw(g, item, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_OR_PLANESWALKER], effect),
    ..oracle::TAKE_OUT_THE_TRASH
};
