use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever a Rat you control becomes blocked" (CR 509.1h, 603.2): the creature that became blocked, one of yours that is a Rat
/// (read through the words of this creature's text, CR 612), is the subject; once per Rat that becomes blocked, however many
/// creatures block it. Tattered Ratter is not itself a Rat.
fn rat_you_control_becomes_blocked(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::BecameBlocked { attacker } = event else { return None };
    if q.controller(*attacker) == asker.controller && q.chars(*attacker).has_subtype(q.words(asker.source).subtype(Subtype::Rat)) {
        return Fired::about(Target::Object(*attacker));
    }
    None
}

/// "It gets +2/+0 until end of turn." (CR 611.2a): the Rat the trigger is about, if it is still on the battlefield.
fn it_gets_plus_two_zero(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(rat) = item.subject {
        cantrip_mechanics::pump(g, item.object, rat, 2, 0);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::BECAME_BLOCKED, rat_you_control_becomes_blocked),
        it_gets_plus_two_zero,
        "Whenever a Rat you control becomes blocked, it gets +2/+0 until end of turn.",
    )],
    ..oracle::TATTERED_RATTER
};
