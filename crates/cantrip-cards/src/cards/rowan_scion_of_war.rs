use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}: Spells you cast this turn that are black and/or red cost {X} less to cast, where X is the amount of life you lost this
/// turn" (CR 611.2c, 601.2f, 608.2h): the amount is worked out as the ability resolves and kept (a later loss does not make the
/// spells cheaper). The effect is made for the player and lasts the turn whether or not Rowan stays on the battlefield.
fn black_and_red_spells_cost_less_by_life_lost(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let lost = g.history().life_lost(item.controller);
    g.spells_cost_this_turn_by(item, item.controller, 0, lost);
    Ok(())
}

/// A spell that is black, red or both (CR 105.4) costs the amount of life lost less, of generic mana only (CR 118.7a). The
/// amount is the one the effect fixed (`Query::replacement_value`).
fn black_or_red_spells_less(q: &dyn Query, source: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let words = q.words(source);
    match costed {
        Costed::Spell(spell) if q.chars(spell).colors.has(words.color(Color::Black)) || q.chars(spell).colors.has(words.color(Color::Red)) => {
            CostDelta::less(q.replacement_value())
        }
        _ => CostDelta::NONE,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{T}"),
        black_and_red_spells_cost_less_by_life_lost,
        "{T}: Spells you cast this turn that are black and/or red cost {X} less to cast, where X is the amount of life you lost this turn. Activate only as a sorcery.",
    )
    .sorcery_speed()],
    lasting_cost_changes: &[LastingCostChange {
        text: "Black and/or red spells you cast this turn cost less to cast, by the life you lost.",
        change: black_or_red_spells_less,
        next: None,
    }],
    ..oracle::ROWAN_SCION_OF_WAR
};
