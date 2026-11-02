use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "This land enters tapped. {T}: Add {B}. {2}{B}, {T}, Sacrifice this land: Return target creature card from your graveyard to your hand." (CR 608.2, 701.26a).
fn add_b_2_b_t_enters(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.tapped = true;
    Ok(())
}

/// "{T}: Add {B}." (CR 605.3a, 106.3).
fn add_b_mana(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::B, 1);
    Ok(())
}

/// "{2}{B}, {T}, Sacrifice this land: Return target creature card from your graveyard to your hand." (CR 115.1).
fn return_target_creature_card_graveyard_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if (q.state().obj(id).zone.kind() == ZoneKind::Graveyard)
        && ! q.state().obj(id).token && ((q.chars(id).is(CardType::Creature) && (q
        .state().obj(id).zone.kind() == ZoneKind::Graveyard && asker.controller == q
        .state().obj(id).owner)))
    )
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            TAP,
            Adds {
                pool: ManaPool([0, 0, 1, 0, 0, 0]),
                ..add_by(add_b_mana)
            },
            "{T}: Add {B}.",
        ),
        ability(
                cost("{2}{B}").and(SACRIFICE_THIS).and(TAP),
                effects::return_first_target_to_hand,
                "{2}{B}, {T}, Sacrifice this land: Return target creature card from your graveyard to your hand.",
            )
            .targets(&[target(return_target_creature_card_graveyard_target_0)]),
    ],
    enters: Some(add_b_2_b_t_enters),
    ..oracle::MEMORIAL_TO_FOLLY
};
