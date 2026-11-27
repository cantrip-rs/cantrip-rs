use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target player activates a mana ability of each land they control. Then that player loses all unspent mana and you add
/// the mana lost this way."
fn drain_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = g.state().target(item, 0).player() else { return Ok(()) };
    activate_a_mana_ability_of_each_land(g, player, item.object)?;
    g.end_sentence();
    // "Then that player loses all unspent mana" (CR 106.4): all of it, whatever its restriction or duration, and none that
    // a rule makes into other mana instead (Horizon Stone, CR 614.1a). "And you add the mana lost this way": the same amount
    // of each type, produced by Drain Power (CR 106.3), so with no restriction, rider or snow of the mana lost (CR 106.6).
    let lost = g.empty_mana_pool(player);
    for mana in Mana::ALL.into_iter().filter(|&mana| lost.get(mana) > 0) {
        g.add_mana(item.controller, mana, lost.get(mana));
    }
    Ok(())
}

/// "Target player activates a mana ability of each land they control" (CR 117.2e, 605.3a). The lands are those they control
/// as the instruction begins (CR 608.2h): one that arrives meanwhile is not among them, and one that has left is a new
/// object (CR 400.7), no longer on the battlefield to be activated. A mana ability of each is activated once, the player
/// choosing which land next, which of its mana abilities and its color, mode or amount (the rulings of 2004-10-04). A land
/// whose mana ability was activated meanwhile, paying another land's mana cost with it (CR 605.3a), has had one activated,
/// as the history says; the instruction ends once no land still owed has a mana ability that can be activated - tapped,
/// none, a cost that can't be paid (CR 609.3).
fn activate_a_mana_ability_of_each_land(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Result<(), Suspend> {
    let lands: Vec<ObjectId> = {
        let q: &dyn Query = g;
        q.permanents_of(player).into_iter().filter(|&id| q.chars(id).is(CardType::Land)).collect()
    };
    let before = g.history().all().len();
    loop {
        let owed: Vec<ObjectId> = {
            let since = &g.history().all()[before..];
            lands
                .iter()
                .copied()
                .filter(|&land| !since.iter().any(|r| matches!(r.event, Event::AbilityActivated { source, .. } if source == land)))
                .collect()
        };
        if g.activate_a_mana_ability_of(player, source, &owed)?.is_none() {
            return Ok(());
        }
    }
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER], drain_power),
    ..oracle::DRAIN_POWER
};
