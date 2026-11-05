use cantrip_api::*;
use cantrip_mechanics::{goyf, targets};
use cantrip_oracle as oracle;

/// "Whenever this creature or another Lhurgoyf creature you control enters": one ability with two conditions, so its
/// own entry counts once. The entered permanent is the ability's subject, "that creature".
fn lhurgoyf_enters(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if to.is(ZoneKind::Battlefield) => {
            if *new == source {
                return Fired::about(Target::Object(*new));
            }
            let chars = q.chars(*new);
            if chars.is(CardType::Creature) && chars.has_subtype(q.words(source).subtype(Subtype::Lhurgoyf)) && q.controller(*new) == q.controller(source) {
                Fired::about(Target::Object(*new))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// That creature deals damage equal to its power to any target (CR 113.7a): the entered creature is the source, with
/// its power as last known if it has left (CR 608.2h). A power of 0 deals no damage at all (CR 120.8).
fn damage_by_the_entered_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(dealer) = item.subject else { return Ok(()) };
    let power = g.chars(dealer).power.unwrap_or(0);
    if power > 0 {
        g.damage(dealer, item.target(0), i64::from(power as u16));
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, lhurgoyf_enters), damage_by_the_entered_creature, "Whenever this creature or another Lhurgoyf creature you control enters, that creature deals damage equal to its power to any target.")
        .targets(&[targets::ANY])],
    statics: &[goyf::SIZE],
    ..oracle::PYROGOYF
};
