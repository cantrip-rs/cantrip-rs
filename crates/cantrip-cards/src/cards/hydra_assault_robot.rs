use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// "Whenever another Villain and/or artifact you control enters" (CR 603.6a): a permanent other than this one, under your
/// control as it entered, that is a Villain, an artifact or both - one event, so it triggers once however many of those it is.
/// The creature type is read through the words of this creature's text (CR 612).
fn another_villain_or_artifact_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if *new != me.source && to.is(ZoneKind::Battlefield) && q.controller(*new) == me.controller => {
            let entered = q.chars(*new);
            if entered.is(CardType::Artifact) || entered.has_subtype(q.words(me.source).subtype(Subtype::Villain)) {
                Fired::PLAIN
            } else {
                None
            }
        }
        _ => None,
    }
}

/// "This creature deals 1 damage to target opponent."
fn ping(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_villain_or_artifact_enters),
        ping,
        "Whenever another Villain and/or artifact you control enters, this creature deals 1 damage to target opponent.",
    )
    .targets(&[targets::OPPONENT])],
    ..oracle::HYDRA_ASSAULT_ROBOT
};
