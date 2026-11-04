use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// I, II - "You may discard a card. If you do, draw a card." (CR 701.9): `effects::may_discard_to_draw`.
fn rummage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_discard_to_draw(g, item, 1)
}

/// III, IV - "Add {R}." (CR 106.4, ruling 2025-06-06): a chapter ability, so it uses the stack; it triggers as the Saga's lore
/// counter is added in the precombat main phase (CR 714.3c), so the mana can be spent there, and empties at the end of that phase.
fn add_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::R, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, rummage, "I \u{2014} You may discard a card. If you do, draw a card."),
        chapter(2, rummage, "II \u{2014} You may discard a card. If you do, draw a card."),
        chapter(3, add_red, "III \u{2014} Add {R}."),
        chapter(4, add_red, "IV \u{2014} Add {R}."),
    ],
    ..oracle::SUMMON_G_F_IFRIT
};
