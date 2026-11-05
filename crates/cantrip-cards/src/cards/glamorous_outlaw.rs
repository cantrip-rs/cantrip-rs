use cantrip_api::*;
use cantrip_mechanics::land_until_cast;
use cantrip_mechanics::scry;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this creature enters, it deals 2 damage to each opponent and you scry 2." The damage is dealt to every opponent
/// at once (CR 120.2), then its controller scries (CR 701.22).
fn enters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hits: Vec<(ObjectId, Target, i64)> = g.state().opponents(item.controller).map(|p| (item.object, Target::Player(p), 2)).collect();
    g.damage_together(&hits);
    scry(g, item.controller, item.object, 2)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, enters, "When this creature enters, it deals 2 damage to each opponent and you scry 2."),
        land_until_cast::ability::<0>(
            cost("{2}"),
            "{2}, Exile this card from your hand: Target land gains \"{T}: Add {U}, {B}, or {R}\" until this card is cast from exile. You may cast this card for as long as it remains exiled.",
        ),
    ],
    delayed: &[mana_ability(TAP, add_one_of(&[Color::Blue, Color::Black, Color::Red]), "{T}: Add {U}, {B}, or {R}.")],
    ..oracle::GLAMOROUS_OUTLAW
};
