use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Tap all creatures your opponents control" (CR 701.26a, 101.4): every creature under an opponent's control as the ability
/// resolves, tapped together; an already-tapped one stays tapped.
fn tap_opponents_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let creatures: Vec<ObjectId> =
        opponents.into_iter().flat_map(|p| g.permanents_of(p)).filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    g.tap_together(&creatures);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, tap_opponents_creatures, "When Ultimecia enters, tap all creatures your opponents control."),
        triggered(when_asked(EventMask::DAMAGE, triggers::creature_you_control_deals_combat_damage_to_player), |g, item| effects::draw(g, item, 1), "Whenever a creature you control deals combat damage to a player, draw a card."),
    ],
    ..oracle::ULTIMECIA_TEMPORAL_THREAT
};
