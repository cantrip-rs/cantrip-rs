use cantrip_api::*;
use cantrip_mechanics::{effects, triggers, your_turn};
use cantrip_oracle as oracle;

/// "Whenever you commit a crime during your turn" (CR 700.13, 603.2): `triggers::you_commit_a_crime`, while it is the source's
/// controller's turn. A crime committed on an opponent's turn does not trigger it.
fn you_commit_a_crime_during_your_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    your_turn(q, me).then(|| triggers::you_commit_a_crime(q, me, event)).flatten()
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::CRIME, you_commit_a_crime_during_your_turn),
        effects::this_gains_indestructible,
        "Whenever you commit a crime during your turn, this creature gains indestructible until end of turn.",
    )],
    ..oracle::OVERZEALOUS_MUSCLE
};
