use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::this_dies};
use cantrip_oracle as oracle;

/// "When this creature is exiled from the battlefield while you're activating a craft ability" (CR 702.167a, 602.2, 603.2): it left
/// the battlefield for exile as its controller's activated ability with materials - craft - was being activated, whether as one of
/// the materials or because a mana ability activated to pay for it moved it (the 2023-11-10 rulings). A card has a new object
/// per zone (CR 400.7), so `old == source` already says the Gnome left the battlefield, not a graveyard.
fn exiled_while_you_activate_a_craft_ability(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, to, .. } if *old == source && to.is(ZoneKind::Exile) => q
            .activation_under_way()
            .filter(|a| a.player == q.obj(source).controller && q.activated_ability(a.ability).is_some_and(|d| d.cost.materials.is_some()))
            .and(Fired::PLAIN),
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(when(EventMask::ZONE_CHANGE, this_dies), effects::gain_life_then_draw::<1, 1>, "When this creature dies, you gain 1 life and draw a card."),
        triggered(
            when(EventMask::ZONE_CHANGE, exiled_while_you_activate_a_craft_ability),
            effects::gain_life_then_draw::<1, 1>,
            "When this creature is exiled from the battlefield while you're activating a craft ability, you gain 1 life and draw a card.",
        ),
    ],
    ..oracle::MARKET_GNOME
};
