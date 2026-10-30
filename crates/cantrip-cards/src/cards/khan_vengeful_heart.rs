use cantrip_api::*;
use cantrip_mechanics::{effects, select, triggers};
use cantrip_oracle as oracle;

/// "When Khan enters, you may sacrifice another creature or artifact. If you do, draw two cards." (CR 603.6a, 608.2d):
/// declining is the empty answer to an up-to-one choice, nothing is asked when Khan is the only permanent its controller
/// has (CR 701.21a), and the cards are drawn only for a permanent actually sacrificed (CR 608.2c). "Another" is any
/// permanent but Khan himself, of either type.
fn sacrifice_another_to_draw_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let rule = select::another_creature_or_artifact_permanents(select::up_to(1));
    if !effects::may_sacrifice(g, you, item.object, rule)?.is_empty() {
        effects::draw(g, item, 2)?;
    }
    Ok(())
}

/// "You may cast this card from exile" (CR 113.6e, 601.3): a static ability that functions while Khan is in exile, so it is
/// his own permission there - the engine reads a card's `exile_casts` off the card itself in exile, not off a permanent
/// (`cantrip-rules`' `own_exile_casts`) - and it is his controller who may cast him by it. It is a permission and no cost of
/// its own: he is cast for his mana cost, or an alternative cost another object gives.
fn may_cast_from_exile(_: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> bool {
    card == asker.source && player == asker.controller
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        sacrifice_another_to_draw_two,
        "When Khan enters, you may sacrifice another creature or artifact. If you do, draw two cards.",
    )],
    exile_casts: &[ExileCast {
        text: "Cast from exile",
        allows: may_cast_from_exile,
        additional: Cost::NONE,
        any_type: false,
        lands: false,
        life_for_mana_value: false,
    }],
    ..oracle::KHAN_VENGEFUL_HEART
};
