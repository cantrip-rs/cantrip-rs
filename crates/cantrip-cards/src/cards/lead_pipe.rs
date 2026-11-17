use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::{effects, statics, triggers};
use cantrip_oracle as oracle;

/// "{2}, Sacrifice this Equipment: Draw a card." (CR 701.21a, 121.1): the cost sacrifices the Equipment, so the card is drawn
/// whatever it was attached to.
fn draw_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::ATTACHED_CREATURE_DIES, effects::each_opponent_loses_1_life, "Whenever equipped creature dies, each opponent loses 1 life."),
        ability(cost("{2}").and(SACRIFICE_THIS), draw_a_card, "{2}, Sacrifice this Equipment: Draw a card."),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, statics::plus_pt::<2, 0>, "Equipped creature gets +2/+0.")],
    ..oracle::LEAD_PIPE
};
