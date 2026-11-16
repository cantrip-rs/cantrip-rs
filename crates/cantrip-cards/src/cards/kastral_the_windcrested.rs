use cantrip_api::*;
use cantrip_mechanics::select::{self, up_to, Selection};
use cantrip_mechanics::triggers::subtype_you_control_deal_combat_damage_to_player;
use cantrip_mechanics::{choose, effects, lands};
use cantrip_oracle as oracle;

/// "You may put a Bird creature card from your hand or graveyard onto the battlefield with a finality counter on it" (CR 608.2c,
/// 122.6, 614.1c): the player sees the Bird creature cards in both zones and may choose none; the finality counter is on the
/// card as it enters. "Bird" is a word of Kastral's text (CR 612), a card as it is in the zone it is in.
fn put_a_bird_from_hand_or_graveyard_with_finality(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let cards: Vec<ObjectId> = g.state().zone(Zone::hand(you)).iter().chain(g.state().zone(Zone::graveyard(you)).iter()).copied().collect();
    let rule = Selection::new(up_to(1)).with(select::CREATURE).with(select::subtype_card(Subtype::Bird)).called("Bird creature");
    if let Some(&bird) = choose::select(g, you, item.object, Purpose::PutOntoBattlefield, cards, rule)?.first() {
        g.put_onto_battlefield_with_counters(bird, you, false, &[(CounterKind::Finality, 1)])?;
    }
    Ok(())
}

/// "Draw a card." (CR 121.1)
fn draw_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)
}

/// "Choose one" (CR 603.3c, 700.2).
static MODAL: ModalDef = modes(&[
    mode(put_a_bird_from_hand_or_graveyard_with_finality, "You may put a Bird creature card from your hand or graveyard onto the battlefield with a finality counter on it."),
    mode(effects::plus_one_counter_on_each_of_subtype_you_control::<{ lands::sub(Subtype::Bird) }>, "Put a +1/+1 counter on each Bird you control."),
    mode(draw_a_card, "Draw a card."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        when_asked(EventMask::DAMAGE, subtype_you_control_deal_combat_damage_to_player::<{ lands::sub(Subtype::Bird) }>),
        &MODAL,
        "Whenever one or more Birds you control deal combat damage to a player, choose one \u{2014} \u{2022} You may put a Bird creature card from your hand or graveyard onto the battlefield with a finality counter on it. \u{2022} Put a +1/+1 counter on each Bird you control. \u{2022} Draw a card.",
    )
    .sums_per_subject()],
    ..oracle::KASTRAL_THE_WINDCRESTED
};
