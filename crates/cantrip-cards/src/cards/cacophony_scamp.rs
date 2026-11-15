use cantrip_api::*;
use cantrip_mechanics::effects::{ask_may, this_deals_damage_equal_to_its_power_to_first_target};
use cantrip_mechanics::triggers::{DEALS_COMBAT_DAMAGE_TO_PLAYER, THIS_DIES};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("SacrificeToProliferate", "sacrifice this creature to proliferate?", "Chosen as Cacophony Scamp's ability resolves", "Sacrifice", "Don't");

/// "Whenever this creature deals combat damage to a player, you may sacrifice it. If you do, proliferate." The choice is made
/// as the ability resolves (CR 608.2d), and is not offered when you can't sacrifice it: gone, or another player's now (CR
/// 701.21a). If the Scamp was sacrificed, its controller proliferates (CR 701.34a) with no player able to respond in between
/// (ruling 2023-02-04).
fn proliferate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    if g.can_sacrifice(you, item.object) && ask_may(g, you, item.object, Purpose::Ask(&ASK))? && !g.sacrifice(you, &[item.object]).is_empty() {
        g.end_sentence();
        g.proliferate(you, item.object)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(DEALS_COMBAT_DAMAGE_TO_PLAYER, proliferate, "Whenever this creature deals combat damage to a player, you may sacrifice it. If you do, proliferate."),
        triggered(THIS_DIES, this_deals_damage_equal_to_its_power_to_first_target, "When this creature dies, it deals damage equal to its power to any target.").targets(&[targets::ANY]),
    ],
    ..oracle::CACOPHONY_SCAMP
};
