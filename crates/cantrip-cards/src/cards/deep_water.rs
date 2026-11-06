use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{U}: Until end of turn, if you tap a land you control for mana, it produces {U} instead of any other type." (CR 614.1a).
fn made_1_applies(
    q: &dyn Query,
    asker: &Asker,
    _: ObjectId,
    proposal: &Proposal,
) -> bool {
    let Proposal::ManaProduction(m) = proposal else { return false };
    let id = m.permanent;
    if (q.chars(id).is(CardType::Land) && asker.controller == q.controller(id))
        && m.player == asker.controller
    {
        return true;
    }
    false
}

/// "{U}: Until end of turn, if you tap a land you control for mana, it produces {U} instead of any other type." (CR 614.1a).
fn made_1_apply(_: &dyn Query, _: &Asker, _: ObjectId, proposal: &mut Proposal) {
    let seen = *proposal;
    let Proposal::ManaProduction(_) = &seen else { return };
    let Proposal::ManaProduction(m) = proposal else { return };
    let total = m.total();
    m.mana = ManaPool::default();
    m.mana.0[Mana::U as usize] = total;
}

/// "{U}: Until end of turn, if you tap a land you control for mana, it produces {U} instead of any other type." (CR 608.2).
fn if_tap_land_control_mana(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_player_effect(
        item.controller,
        item.object,
        EffectKind::ReplacementRule { card: item.ability.card, index: 0 },
        Duration::EndOfTurn,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{U}"),
            if_tap_land_control_mana,
            "{U}: Until end of turn, if you tap a land you control for mana, it produces {U} instead of any other type.",
        ),
    ],
    granted_replacements: &[
        granted_replacement(
                ReplacementKind::ManaProduction,
                made_1_applies,
                made_1_apply,
                "{U}: Until end of turn, if you tap a land you control for mana, it produces {U} instead of any other type.",
            )
            .text(
                "Until end of turn, if you tap a land you control for mana, it produces {U} instead of any other type.",
            ),
    ],
    ..oracle::DEEP_WATER
};
