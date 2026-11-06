use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{9}: Untap this creature. Activate only during your upkeep." (CR 602.5).
fn untap_creature_activate_only_during_if(q: &dyn Query, asker: &Asker) -> bool {
    Some(asker.controller)
        .is_some_and(|player: PlayerId| {
            q.state().turn.active == player && q.state().turn.step() == Step::Upkeep
        })
}

pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::Trample, Keyword::DoesNotUntap]),
    abilities: &[
        ability(
                cost("{9}"),
                cantrip_mechanics::effects::untap_self,
                "{9}: Untap this creature. Activate only during your upkeep.",
            )
            .condition(untap_creature_activate_only_during_if),
    ],
    ..oracle::COLOSSUS_OF_SARDIA
};
