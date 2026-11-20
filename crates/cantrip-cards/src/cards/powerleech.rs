use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever an artifact an opponent controls becomes tapped or an opponent activates an artifact's ability without {T} in its
/// activation cost" (CR 603.2, 602.2): one triggered ability with two conditions, listening to `TAPPED` and `ABILITY_ACTIVATED`.
fn opponent_artifact_tapped_or_activated(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    if let Event::Tapped { permanent, .. } = *event {
        return if q.chars(permanent).is(CardType::Artifact)
            && q.state().opponents(asker.controller).any(|p| p == q.controller(permanent))
        {
            Fired::about(Target::Object(permanent))
        } else {
            None
        };
    }
    let (source, player, def) = triggers::activation(q, event)?;
    if q.state().opponents(asker.controller).any(|p| p == player)
        && def.from == ZoneKind::Battlefield && q.chars(source).is(CardType::Artifact) && !def.cost.tap
    {
        Fired::about(Target::Player(player))
    } else {
        None
    }
}

fn gain_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_life(g, item, 1)
}

const TEXT: &str = "Whenever an artifact an opponent controls becomes tapped or an opponent activates an artifact's ability without {T} in its activation cost, you gain 1 life.";
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask(EventMask::TAPPED.0 | EventMask::ABILITY_ACTIVATED.0), opponent_artifact_tapped_or_activated),
        gain_one,
        TEXT,
    )],
    ..oracle::POWERLEECH
};
