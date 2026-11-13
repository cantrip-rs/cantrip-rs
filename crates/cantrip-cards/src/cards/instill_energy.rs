use cantrip_api::*;
use cantrip_mechanics::{attach, targets};
use cantrip_oracle as oracle;

/// "Enchanted creature can attack as though it had haste. {0}: Untap enchanted creature. Activate only during your turn and only once each turn." (CR 604.1).
fn untap_enchanted_creature_activate_only_rule(
    q: &dyn Query,
    asker: &Asker,
    attacker: ObjectId,
    _: Target,
) -> bool {
    attach::attached_to_it(q, asker, attacker)
}

/// Untap the creature attached on resolution, or the Aura's last known attachment (CR 303.4m, 608.2h, 701.26b).
fn untap_enchanted_creature_activate_only(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    if let Some(creature) = g.obj(item.object).attached_to {
        g.untap(creature);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{0}"),
                untap_enchanted_creature_activate_only,
                "{0}: Untap enchanted creature. Activate only during your turn and only once each turn.",
            )
            .once_each_turn()
            .timing(Timing::OwnTurn),
    ],
    enchant: Some(targets::CREATURE),
    attacks_as_though_haste: &[untap_enchanted_creature_activate_only_rule],
    ..oracle::INSTILL_ENERGY
};
