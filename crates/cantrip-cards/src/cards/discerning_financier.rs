use cantrip_api::*;
use cantrip_mechanics::{an_opponent_controls_more_lands, choose, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "Target Treasure you control" (CR 115.1, 111.10a): a Treasure on the battlefield under the ability's controller, judged again
/// as the ability resolves (CR 608.2b), so one whose control changed in response is no longer legal.
fn treasure_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::permanent_you_control_filter(q, asker, t) && matches!(t, Target::Object(id) if q.chars(id).has_subtype(Subtype::Treasure))
}

/// "Choose another player. That player gains control of target Treasure you control. You draw a card." Each instruction in the
/// order written (CR 608.2c): the player is chosen without targeting (CR 608.2d, 115.10a) among those other than you, then takes
/// the Treasure (CR 613.1b, 611.2a: with no duration, for the rest of the game), then you draw. With the one target illegal the ability does not resolve
/// (CR 608.2b), so you draw nothing; with no other player to choose, the Treasure stays and you still draw.
fn hand_over_a_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(treasure) = targets::object(g, item, 0) else { return Ok(()) };
    let others: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    if let Some(player) = choose::player_about(g, item.controller, item.object, others, "GainControlOf", treasure)? {
        g.gain_control(item.object, treasure, player, Duration::Indefinite);
    }
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            tokens::create_a_treasure,
            "At the beginning of your upkeep, if an opponent controls more lands than you, create a Treasure token.",
        )
        .condition(an_opponent_controls_more_lands),
        ability(
            cost("{2}{W}"),
            hand_over_a_treasure,
            "{2}{W}: Choose another player. That player gains control of target Treasure you control. You draw a card.",
        )
        .targets(&[target(treasure_you_control)]),
    ],
    ..oracle::DISCERNING_FINANCIER
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
