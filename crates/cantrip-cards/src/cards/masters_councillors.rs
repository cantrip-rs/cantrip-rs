use cantrip_api::*;
use cantrip_mechanics::statics::itself;
use cantrip_mechanics::triggers::YOU_DRAW_YOUR_SECOND_CARD;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "This creature gets +2/+0 for each graveyard with seven or more cards in it." (CR 613.4c, 404.1): every player's graveyard
/// counts, counted as the layers are worked out.
fn plus_two_for_each_big_graveyard(view: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    let state = view.state();
    let big = state.seats().filter(|&player| state.zone(Zone::graveyard(player)).len() >= 7).count() as i16;
    chars.power = chars.power.map(|v| v + 2 * big);
}

/// "Target player mills three cards." (CR 701.17a): nothing if the target is gone (CR 608.2b).
fn mill_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    g.mill(player, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        itself,
        plus_two_for_each_big_graveyard,
        "This creature gets +2/+0 for each graveyard with seven or more cards in it.",
    )],
    abilities: &[triggered(YOU_DRAW_YOUR_SECOND_CARD, mill_three, "Whenever you draw your second card each turn, target player mills three cards.")
        .targets(&[targets::PLAYER])],
    ..oracle::MASTERS_COUNCILLORS
};
