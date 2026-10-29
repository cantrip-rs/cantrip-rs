use cantrip_api::*;
use cantrip_mechanics::haste_until_end_of_turn;
use cantrip_oracle as oracle;

/// "Gain control of all artifacts your opponents control until end of turn. Untap them. They gain haste until end of turn."
/// (CR 613.1b, 611.2a, 611.2c, 702.10): the artifacts are those an opponent controls as the spell resolves, and the effects
/// affect just them, not an artifact that comes under an opponent's control later in the turn. Three sentences, each judged
/// by the state triggers before the next (CR 603.8, 608.2c). One you control already is no opponent's, so it is left alone.
fn take_over(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let artifacts: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Artifact) && opponents.contains(&g.controller(id)))
        .collect();
    instruction(g, |g| {
        for &artifact in &artifacts {
            g.gain_control(item.object, artifact, item.controller, Duration::EndOfTurn);
        }
    });
    g.end_sentence();
    for &artifact in &artifacts {
        g.untap(artifact);
    }
    g.end_sentence();
    instruction(g, |g| {
        for &artifact in &artifacts {
            haste_until_end_of_turn(g, item.object, artifact);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], take_over),
    ..oracle::BROADCAST_TAKEOVER
};
