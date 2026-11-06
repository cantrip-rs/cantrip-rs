use cantrip_api::*;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target nonland permanent card from that player's graveyard": a nonland permanent card (CR 110.4a) in the graveyard of the
/// player the combat damage was dealt to (the trigger's subject).
fn nonland_permanent_card_in_that_players_graveyard(q: &dyn Query, _: &Asker, t: Target) -> bool {
    match (t, q.event_subject()) {
        (Target::Object(id), Target::Player(p)) => {
            targets::card_in_graveyard(q, t, &targets::PERMANENT_CARD_TYPES)
                && targets::is_nonland_permanent_card(&q.chars(id))
                && q.obj(id).zone == Zone::graveyard(p)
        }
        _ => false,
    }
}

/// "You may cast target nonland permanent card from that player's graveyard, and mana of any type can be spent to cast that
/// spell." (CR 608.2g, 609.4b.) Cast as the ability resolves, ignoring the timing restrictions of its types (the ruling). The
/// restriction is on the card, not the spell, so an Adventure or Omen face may be the one cast (CR 715.3, 720.3). A cast that
/// cannot be paid for is given up (CR 733.1).
fn cast_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_cast_first_target(g, item, None, true, ExileInstead::Never)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT, cast_it, "Whenever Tinybones deals combat damage to a player, you may cast target nonland permanent card from that player's graveyard, and mana of any type can be spent to cast that spell.")
        .targets(&[target(nonland_permanent_card_in_that_players_graveyard)])],
    ..oracle::TINYBONES_THE_PICKPOCKET
};
