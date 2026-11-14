use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Enchanted player can't gain life" (CR 119.7, 303.4): the player this Aura is attached to.
fn enchanted_player_cant_gain_life(q: &dyn Query, asker: &Asker, player: PlayerId) -> bool {
    q.obj(asker.source).attached_player == Some(player)
}

/// "Whenever enchanted player is dealt damage" (CR 120.2, 603.2): damage (never of 0, CR 120.8) to the player this Aura is attached to,
/// who is the trigger's subject ("they"). The ability is `.sums_per_subject()`: damage dealt all at once triggers it once.
fn enchanted_player_dealt_damage(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { target: Target::Player(p), .. } if q.obj(source).attached_player == Some(*p) => Fired::about(Target::Player(*p)),
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    cant_gain_life: &[enchanted_player_cant_gain_life],
    abilities: &[triggered(
        when(EventMask::DAMAGE, enchanted_player_dealt_damage),
        effects::subject_player_loses_half_their_life,
        "Whenever enchanted player is dealt damage, they lose half their life, rounded up.",
    )
    .sums_per_subject()],
    enchant: Some(targets::PLAYER),
    ..oracle::GRIEVOUS_WOUND
};
