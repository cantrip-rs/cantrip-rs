use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever you play a land" (CR 305.1): a land of yours put onto the battlefield by the special action, not by an
/// effect.
fn you_play_a_land(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, reason: ZoneReason::PlayLand, .. }
            if to.is(ZoneKind::Battlefield) && q.controller(*new) == q.controller(source) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "If it wasn't the first land you played this turn" (CR 603.4): the turn's count already includes this land.
fn not_the_first(q: &dyn Query, asker: &Asker) -> bool {
    q.state().player(asker.controller).lands_played > 1
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, you_play_a_land), cantrip_mechanics::effects::source_deals_one_to_controller, "Whenever you play a land, if it wasn't the first land you played this turn, this enchantment deals 1 damage to you.")
        .condition(not_the_first)],
    play_rules: &[PlayRule::AnyNumberOfLandPlays],
    ..oracle::FASTBOND
};
