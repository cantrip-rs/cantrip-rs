use cantrip_api::*;
use cantrip_mechanics::{choose, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// Each player exiles cards from the top of their library until they exile a nonland card, from the active player on
/// (CR 101.4). You may cast any number of spells from among the nonland cards exiled this way without paying their mana costs.
fn exile_and_cast_free(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut found: Vec<ObjectId> = Vec::new();
    for player in g.state().apnap().collect::<Vec<_>>() {
        found.extend(cantrip_mechanics::library::exile_until_any_nonland(g, player, None));
    }
    choose::cast_any_number_free(g, item.controller, item.object, &found)
}

fn give_poison(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Player(p) = item.subject {
        g.give_poison(p, item.data[0]);
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT, give_poison, "Whenever Etali deals combat damage to a player, they get that many poison counters.")],
    ..oracle::ETALI_PRIMAL_SICKNESS
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, exile_and_cast_free, "When Etali enters, each player exiles cards from the top of their library until they exile a nonland card. You may cast any number of spells from among the nonland cards exiled this way without paying their mana costs."),
        ability(cost("{9}{G/P}"), cantrip_mechanics::effects::transform_self, "{9}{G/P}: Transform Etali. Activate only as a sorcery.")
            .sorcery_speed(),
    ],
    back: Some(&BACK),
    ..oracle::ETALI_PRIMAL_CONQUEROR
};
