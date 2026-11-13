use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "{T}, Exile a card from your graveyard: Add {R}. When you do, this creature deals 1 damage to each opponent." The ability is a
/// mana ability (CR 605.1a) and adds its mana at once; its reflexive trigger (CR 603.12, 605.5a) is not one, so it waits for the
/// stack until the cast or activation being paid for is done, and players may respond to it.
fn add_red_then_reflex(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::R, 1);
    let this = g.card_id(&CARD);
    g.reflexive_trigger(asker.source, asker.controller, AbilityRef::delayed(this, 0), Target::None);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| effects::may_discard_to_draw(g, item, 1), "When this creature enters, you may discard a card. If you do, draw a card."),
        mana_ability(
            TAP.and(exile_from_graveyard(1)),
            add_by(add_red_then_reflex),
            "{T}, Exile a card from your graveyard: Add {R}. When you do, this creature deals 1 damage to each opponent.",
        ),
    ],
    delayed: &[delayed_trigger(effects::damage_each_opponent_n::<1>, "When you do, this creature deals 1 damage to each opponent.")],
    ..oracle::RUBBLE_ROUSER
};
