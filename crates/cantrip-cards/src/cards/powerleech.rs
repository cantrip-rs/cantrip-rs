use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// An artifact an opponent controls becomes tapped (CR 603.2).
fn opponent_artifact_tapped(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::Tapped { permanent } = event else { return None };
    if q.chars(*permanent).is(CardType::Artifact)
        && q.state().opponents(asker.controller).any(|p| p == q.controller(*permanent))
    {
        Fired::about(Target::Object(*permanent))
    } else { None }
}

/// An opponent activates an artifact's ability without {T} in its cost (CR 602.2, 603.2).
fn opponent_activates_artifact_without_tap(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let (source, player, def) = triggers::activation(q, event)?;
    if q.state().opponents(asker.controller).any(|p| p == player)
        && def.from == ZoneKind::Battlefield && q.chars(source).is(CardType::Artifact) && !def.cost.tap
    {
        Fired::about(Target::Player(player))
    } else { None }
}

fn gain_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_life(g, item, 1)
}

const TEXT: &str = "Whenever an artifact an opponent controls becomes tapped or an opponent activates an artifact's ability without {T} in its activation cost, you gain 1 life.";
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(when_asked(EventMask::TAPPED, opponent_artifact_tapped), gain_one, TEXT),
        triggered(when_asked(EventMask::ABILITY_ACTIVATED, opponent_activates_artifact_without_tap), gain_one, TEXT),
    ],
    ..oracle::POWERLEECH
};
