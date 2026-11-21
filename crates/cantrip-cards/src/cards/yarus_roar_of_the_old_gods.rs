use cantrip_api::*;
use cantrip_mechanics::triggers::{creature_you_control_deals_combat_damage_to_player, creature_you_control_dies};
use cantrip_mechanics::{effects, statics, targets};
use cantrip_oracle as oracle;

/// "Whenever one or more face-down creatures you control deal combat damage to a player" (CR 510.2, 708.2): once for each
/// player they dealt it to, however many of them did (the ruling), the creature face down as it dealt the damage.
fn face_down_creatures_deal_combat_damage_to_a_player(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source, .. } if q.obj(*source).face_down => creature_you_control_deals_combat_damage_to_player(q, me, event),
        _ => None,
    }
}

/// "Whenever a face-down creature you control dies" (CR 700.4, 603.10a): the creature as it last was on the battlefield, face
/// down there. The trigger is about it, so its card in the graveyard is found as it resolves.
fn face_down_creature_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, .. } if q.obj(*old).face_down => {
            creature_you_control_dies(q, me, event)?;
            Fired::about(Target::Object(*old))
        }
        _ => None,
    }
}

/// "Return it to the battlefield face down under its owner's control if it's a permanent card, then turn it face up." The
/// card the creature became in the graveyard, nothing if it has left there since (the ruling; CR 400.7), put back face down
/// neither manifested nor cloaked (CR 708.2a, `FaceDown::Bare`), then turned face up by this ability's controller (CR
/// 708.8). One that can't be turned face up stays a plain 2/2 face down, with no ward (the ruling).
fn return_it_face_down_then_turn_it_face_up(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(died) = item.subject else { return Ok(()) };
    let cards: Vec<ObjectId> = g
        .found_all_after_move(died)
        .into_iter()
        .filter(|&card| g.obj(card).zone.is(ZoneKind::Graveyard) && targets::is_permanent_card(&g.chars(card)))
        .collect();
    // Every card of a merged creature comes back (CR 730.3c), each under its own owner's control, together per owner.
    let mut owners: Vec<PlayerId> = Vec::new();
    for &card in &cards {
        let owner = g.obj(card).owner;
        if !owners.contains(&owner) {
            owners.push(owner);
        }
    }
    for owner in owners {
        let group: Vec<ObjectId> = cards.iter().copied().filter(|&card| g.obj(card).owner == owner).collect();
        for permanent in g.put_all_onto_battlefield_face_down(&group, owner, FaceDown::Bare, false)?.into_iter().flatten() {
            g.turn_face_up(permanent, item.controller);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::DAMAGE, face_down_creatures_deal_combat_damage_to_a_player),
            |g, item| effects::draw(g, item, 1),
            "Whenever one or more face-down creatures you control deal combat damage to a player, draw a card.",
        )
        .sums_per_subject(),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, face_down_creature_you_control_dies),
            return_it_face_down_then_turn_it_face_up,
            "Whenever a face-down creature you control dies, return it to the battlefield face down under its owner's control if it's a permanent card, then turn it face up.",
        ),
    ],
    statics: &[static_ability(Layer::Ability, statics::other_creatures_you_control, statics::have_haste, "Other creatures you control have haste.")],
    ..oracle::YARUS_ROAR_OF_THE_OLD_GODS
};
