use cantrip_api::*;
use cantrip_mechanics::{effects, lands, tokens};
use cantrip_oracle as oracle;

/// "This spell costs {X} less to cast, where X is the number of differently named lands you control" (CR 601.2f,
/// 118.7a): a generic reduction of the spell's casting cost, with the land count read as the total cost is determined.
fn cost_reduction(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    let count = lands::differently_named_lands(q, player);
    CostDelta::less(count.min(255) as u8)
}

/// Create the two Romulans, then draw two cards and discard one (CR 111.1, 121.1, 701.9).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::ROMULAN, 2)?;
    g.end_sentence();
    effects::draw_two_then_discard(g, item)
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[cost_reduction],
    spell: spell(&[], effect),
    ..oracle::CLOAK_AND_PHASER
};

pub static EXTRA: &[&CardDef] = &[&tokens::ROMULAN];
