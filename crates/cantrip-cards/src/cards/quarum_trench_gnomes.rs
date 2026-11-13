use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{T}: If target Plains is tapped for mana, it produces colorless mana instead of white mana. (This effect lasts indefinitely.)" (CR 614.1a).
fn produces_1_applies(
    q: &dyn Query,
    asker: &Asker,
    affected: ObjectId,
    proposal: &Proposal,
) -> bool {
    matches!(
        proposal, Proposal::ManaProduction(m) if m.permanent == affected && m.mana
        .get(q.words(asker.source).color(Color::White).mana()) > 0
    )
}

/// "{T}: If target Plains is tapped for mana, it produces colorless mana instead of white mana. (This effect lasts indefinitely.)" (CR 614.1a).
fn produces_1_apply(q: &dyn Query, asker: &Asker, _: ObjectId, proposal: &mut Proposal) {
    let Proposal::ManaProduction(m) = proposal else { return };
    let color = q.words(asker.source).color(Color::White).mana();
    let n = std::mem::take(&mut m.mana.0[color as usize]);
    m.mana.0[Mana::C as usize] += n;
}

/// "{T}: If target Plains is tapped for mana, it produces colorless mana instead of white mana." (CR 608.2, 611.2a).
fn if_target_plains_tapped_mana(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(object) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_effect(
        item.object,
        object,
        EffectKind::Replacement {
            card: item.ability.card,
            index: 0,
        },
        Duration::Indefinite,
    );
    Ok(())
}

/// Target Plains (CR 115.1, 205.3i): a permanent with the land type, as the text of this card names it (CR 612.2).
fn target_plains(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::permanent_of_any_subtype(q, asker, t, &[Subtype::Plains])
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                if_target_plains_tapped_mana,
                "{T}: If target Plains is tapped for mana, it produces colorless mana instead of white mana.",
            )
            .targets(&[target(target_plains)]),
    ],
    granted_replacements: &[
        granted_replacement(
                ReplacementKind::ManaProduction,
                produces_1_applies,
                produces_1_apply,
                "produces another type of mana instead",
            )
            .text(
                "If target Plains is tapped for mana, it produces colorless mana instead of white mana.",
            ),
    ],
    ..oracle::QUARUM_TRENCH_GNOMES
};
