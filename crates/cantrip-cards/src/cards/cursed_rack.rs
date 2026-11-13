use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

/// "As this artifact enters, choose an opponent. The chosen player's maximum hand size is four." (CR 614.1c).
fn as_artifact_enters_choose_opponent_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    let player_0 = entering.controller;
    let candidates_0 = {
        let q: &dyn Query = g;
        q.state().opponents(entering.controller).collect::<Vec<PlayerId>>()
    };
    let Some(chosen_0) = choose::player(
        g,
        player_0,
        entering.object,
        candidates_0,
        "has a maximum hand size of four",
        0,
    )? else { return Ok(()) };
    entering.chosen_player = Some(chosen_0);
    Ok(())
}

/// "As this artifact enters, choose an opponent. The chosen player's maximum hand size is four." (CR 402.2).
fn as_artifact_enters_choose_opponent_hand_size_0(
    q: &dyn Query,
    asker: &Asker,
    player: PlayerId,
) -> bool {
    q.state()
        .chosen_player(q.linked_to(asker))
        .into_iter()
        .collect::<Vec<PlayerId>>()
        .contains(&player)
}

pub static CARD: CardDef = CardDef {
    enters: Some(as_artifact_enters_choose_opponent_enters),
    hand_size_rules: &[
        HandSizeRule {
            applies: as_artifact_enters_choose_opponent_hand_size_0,
            size: HandSize::Set(4),
        },
    ],
    ..oracle::CURSED_RACK
};
