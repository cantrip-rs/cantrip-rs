use cantrip_api::*;
use cantrip_mechanics::{artifacts_you_control, you_control_an_enchantment};
use cantrip_oracle as oracle;

/// "{2}, {T}, Sacrifice this land: Draw a card if you control an artifact. Draw a card if you control an enchantment."
/// (CR 608.2c, 121.1): each sentence asks the question again, as it is then.
fn draw_card_if_control_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if artifacts_you_control(&*g, item.controller) > 0 {
        g.draw(item.controller, 1);
    }
    g.end_sentence();
    if you_control_an_enchantment(&*g, item.controller) {
        g.draw(item.controller, 1);
        g.end_sentence();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(
            cost("{2}").and(SACRIFICE_THIS).and(TAP),
            draw_card_if_control_artifact,
            "{2}, {T}, Sacrifice this land: Draw a card if you control an artifact. Draw a card if you control an enchantment.",
        ),
    ],
    ..oracle::ROADSIDE_RELIQUARY
};
