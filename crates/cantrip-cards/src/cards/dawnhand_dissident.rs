use cantrip_api::*;
use cantrip_mechanics::{blight, effects, targets, your_turn};
use cantrip_oracle as oracle;

/// "{T}, Blight 1: Surveil 1." Blight 1 (CR 701.68a) puts a -1/-1 counter on a creature its controller controls as the
/// cost is paid; a player with no creature can't pay it (CR 701.68b).
fn surveil_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 1)
}

/// "{T}, Blight 2: Exile target card from a graveyard." The card is exiled with this creature (CR 607.2a), which the third
/// ability reads: linked to the permanent that activated the ability, so a Dissident that left and came back is a new object
/// that finds none of them (CR 400.7).
fn exile_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(card) = targets::object(g, item, 0) {
        g.exile_with(&[card], g.linked(item), false);
    }
    Ok(())
}

/// "During your turn, you may cast creature spells from among cards you own exiled with this creature": its controller
/// ("you", CR 109.5) on their own turn, a card they own that the second ability exiled (CR 607.2a), cast as a creature
/// spell (a creature card's instant Adventure is not one, CR 715.3a). Timing is the spell's own (the rulings).
fn allows(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> bool {
    player == asker.controller
        && your_turn(q, asker)
        && q.obj(card).owner == player
        && q.exiled_with(q.linked_to(asker)).contains(&card)
        && q.chars(card).is(CardType::Creature)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP.and(blight::cost(1)), surveil_one, "{T}, Blight 1: Surveil 1."),
        ability(TAP.and(blight::cost(2)), exile_card, "{T}, Blight 2: Exile target card from a graveyard.").targets(&[targets::CARD_IN_A_GRAVEYARD]),
    ],
    // "... by removing three counters from among creatures you control in addition to paying their other costs" (CR 118.8,
    // 601.2f): an additional cost on each cast by it, of any kinds of counter, from any of their creatures (CR 118.3).
    exile_casts: &[ExileCast {
        text: "Cast by removing three counters from among creatures you control",
        allows,
        additional: remove_counters_among(3, targets::filters::creature),
        any_type: false,
        lands: false,
        life_for_mana_value: false,
        instead: None,
    }],
    ..oracle::DAWNHAND_DISSIDENT
};
