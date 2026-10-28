use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this creature enters, create a Food token or a Treasure token." (CR 111.10b, 111.10a): its controller chooses which
/// as it resolves. The labels are Ant-Man's Army's own.
fn make_food_or_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = g.card_id(&CARD);
    if choose::one_of(g, item.controller, item.object, vec![Label::Card(card, 0), Label::Card(card, 1)])? == 0 {
        tokens::create_food(g, item.controller)
    } else {
        tokens::create_treasure(g, item.controller)
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, make_food_or_treasure, "When this creature enters, create a Food token or a Treasure token.")],
    labels: &["Food token", "Treasure token"],
    ..oracle::ANT_MANS_ARMY
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD, &tokens::TREASURE];
