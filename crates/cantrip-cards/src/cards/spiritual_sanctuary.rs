use cantrip_api::*;
use cantrip_mechanics::triggers::EACH_PLAYERS_UPKEEP;
use cantrip_oracle as oracle;

/// The upkeep player's Plains condition is checked when the ability triggers and resolves (CR 603.4).
fn upkeep_player_controls_plains(q: &dyn Query, asker: &Asker) -> bool {
    let plains = q.words(asker.source).subtype(Subtype::Plains);
    q.permanents_of(q.state().turn.active).iter().any(|&id| q.chars(id).has_subtype(plains))
}

/// The player whose upkeep began gains one life (CR 119.3).
fn gain_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = item.subject.player() {
        g.gain_life(player, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        EACH_PLAYERS_UPKEEP,
        gain_life,
        "At the beginning of each player's upkeep, if that player controls a Plains, they gain 1 life.",
    ).condition(upkeep_player_controls_plains)],
    ..oracle::SPIRITUAL_SANCTUARY
};
