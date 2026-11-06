use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

/// "As this artifact enters, choose an opponent. At the beginning of the chosen player's upkeep, this artifact deals X damage to that player, where X is 3 minus the number of cards in their hand." (CR 614.1c).
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
        "takes damage at the beginning of their upkeep based on the number of cards in their hand",
        0,
    )? else { return Ok(()) };
    entering.chosen_player = Some(chosen_0);
    Ok(())
}

/// "At the beginning of the chosen player's upkeep" (CR 603.2).
fn when_beginning_chosen_players_upkeep(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::StepBegan { step } = event else { return None };
    let subject = Target::Player(q.state().turn.active);
    if matches!(* step, Step::Upkeep)
        && q.state().chosen_player(q.linked_to(asker)) == Some(q.state().turn.active)
    {
        return Fired::about(subject);
    }
    None
}

/// "At the beginning of the chosen player's upkeep, this artifact deals X damage to that player, where X is 3 minus the number of cards in their hand." (CR 608.2, 120.3).
fn artifact_deals_x_damage_player(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let amount_1 = {
        let q: &dyn Query = g;
        (3
            - item
                .subject
                .player()
                .into_iter()
                .flat_map(|p: PlayerId| q.state().zone(Zone::hand(p)).to_vec())
                .collect::<Vec<ObjectId>>()
                .into_iter()
                .filter(|&id| !q.state().obj(id).token && !q.state().obj(id).card_copy)
                .collect::<Vec<ObjectId>>()
                .len() as i64)
            .max(0)
    };
    let recipients_1 = item
        .subject
        .player()
        .into_iter()
        .map(Target::Player)
        .collect::<Vec<Target>>();
    let batch: Vec<(ObjectId, Target, u32)> = recipients_1
        .iter()
        .map(|t| (item.object, *t, amount_1 as u32))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::STEP_BEGAN, when_beginning_chosen_players_upkeep),
            artifact_deals_x_damage_player,
            "At the beginning of the chosen player's upkeep, this artifact deals X damage to that player, where X is 3 minus the number of cards in their hand.",
        ),
    ],
    enters: Some(as_artifact_enters_choose_opponent_enters),
    ..oracle::THE_RACK
};
