use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "{X}: Destroy each nonland permanent with mana value X whose controller was dealt combat damage by this creature this turn."
/// (CR 120.3, 120.2, 701.8a): the players are read from this turn's damage events, so damage that was prevented names none (CR
/// 615.6) and damage dealt to a planeswalker or a creature names none. Permanents are checked as the ability resolves, so one
/// that changed hands since is judged by its new controller. They are destroyed together.
fn destroy_each_nonland_permanent_with_mana_value_x_whose_controller_was_dealt_combat_damage(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let damaged: Vec<PlayerId> = g
        .history()
        .damage_dealt()
        .filter(|d| d.source == item.object && d.combat && d.amount > 0)
        .filter_map(|d| match d.target {
            Target::Player(p) => Some(p),
            _ => None,
        })
        .collect();
    let doomed: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            !g.chars(id).is(CardType::Land) && g.mana_value(id) == item.x && damaged.contains(&g.controller(id))
        })
        .collect();
    g.destroy_all(&doomed);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{2}"), effects::this_gets::<1, 0>, "{2}: This creature gets +1/+0 until end of turn."),
        ability(
            cost("{X}"),
            destroy_each_nonland_permanent_with_mana_value_x_whose_controller_was_dealt_combat_damage,
            "{X}: Destroy each nonland permanent with mana value X whose controller was dealt combat damage by this creature this turn. Activate only once each turn.",
        )
        .once_each_turn(),
    ],
    ..oracle::STEEL_HELLKITE
};
