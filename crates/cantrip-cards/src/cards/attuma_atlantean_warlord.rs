use cantrip_api::*;
use cantrip_mechanics::statics::{other_of_subtype_you_control, plus_pt};
use cantrip_mechanics::{effects, lands};
use cantrip_oracle as oracle;

/// "Whenever one or more Merfolk you control attack a player" (CR 508.1, 603.2c): a Merfolk under Attuma's controller's
/// control (the type read through the words of Attuma's text, CR 612.2), declared as an attacker of a player rather than a
/// planeswalker or battle. Each player attacked is a batch of its own, so the ability triggers once for each player the
/// Merfolk attack (`batch_by_data` on the defender), whatever their number. A creature put onto the battlefield attacking was
/// never declared as an attacker (CR 508.4) and triggers nothing.
fn merfolk_you_control_attack_a_player(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, defender: Target::Player(player), .. }
            if q.controller(*attacker) == me.controller && q.chars(*attacker).has_subtype(q.words(me.source).subtype(Subtype::Merfolk)) =>
        {
            Some(Fired { subject: Target::Object(*attacker), data: [0, player.0 as u32] })
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        other_of_subtype_you_control::<{ lands::sub(Subtype::Merfolk) }>,
        plus_pt::<1, 1>,
        "Other Merfolk you control get +1/+1.",
    )],
    abilities: &[triggered(
        when_asked(EventMask::ATTACKS, merfolk_you_control_attack_a_player),
        |g, item| effects::draw(g, item, 1),
        "Whenever one or more Merfolk you control attack a player, draw a card.",
    )
    .once_per_batch()
    .batch_by_data()],
    ..oracle::ATTUMA_ATLANTEAN_WARLORD
};
