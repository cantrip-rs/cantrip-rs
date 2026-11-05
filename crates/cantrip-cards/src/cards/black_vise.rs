use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

/// "As this artifact enters, choose an opponent. At the beginning of the chosen player's upkeep, this artifact deals X damage to that player, where X is the number of cards in their hand minus 4." (CR 614.1c).
fn as_artifact_enters_choose_opponent_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    let candidates = g.state().opponents(entering.controller).collect();
    let Some(chosen_0) = choose::player(
        g,
        entering.controller,
        entering.object,
        candidates,
        "takes damage at the beginning of their upkeep equal to the number of cards in their hand minus four",
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

/// "At the beginning of the chosen player's upkeep, this artifact deals X damage to that player, where X is the number of cards in their hand minus 4." (CR 608.2, 120.3).
fn artifact_deals_x_damage_player(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let amount_1 = {
        let q: &dyn Query = g;
        (item
            .subject
            .player()
            .into_iter()
            .flat_map(|p: PlayerId| q.state().zone(Zone::hand(p)).to_vec())
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .collect::<Vec<ObjectId>>()
            .len() as i64 - 4)
            .max(0)
    };
    let recipients_1 = item
        .subject
        .player()
        .into_iter()
        .map(Target::Player)
        .collect::<Vec<Target>>();
    let batch: Vec<(ObjectId, Target, i64)> = recipients_1
        .iter()
        .map(|t| (item.object, *t, amount_1))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::STEP_BEGAN, when_beginning_chosen_players_upkeep),
            artifact_deals_x_damage_player,
            "At the beginning of the chosen player's upkeep, this artifact deals X damage to that player, where X is the number of cards in their hand minus 4.",
        ),
    ],
    enters: Some(as_artifact_enters_choose_opponent_enters),
    ..oracle::BLACK_VISE
};
