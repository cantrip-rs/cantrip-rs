use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_oracle as oracle;

/// "{6}, {T}: Prevent all damage that would be dealt to you this turn by creatures with flying." (CR 615.1).
fn made_1_applies(q: &dyn Query, _: &Asker, _: ObjectId, e: &DamageEvent) -> bool {
    {
        let source = e.source;
        if !q.state().obj(source).zone.is(ZoneKind::Stack)
            && (q.chars(source).is(CardType::Creature)
                && q.chars(source).keywords.has(Keyword::Flying))
        {
            return true;
        }
    }
    false
}

/// "{6}, {T}: Prevent all damage that would be dealt to you this turn by creatures with flying." (CR 608.2, 611.2a).
fn prevent_all_damage_would_be(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    prevention::this_turn(g, item, Target::Player(item.controller), 0, Shield::All);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{6}").and(TAP),
            prevent_all_damage_would_be,
            "{6}, {T}: Prevent all damage that would be dealt to you this turn by creatures with flying.",
        ),
    ],
    granted_preventions: &[
        GrantedPreventionDef {
            hint: "{6}, {T}: Prevent all damage that would be dealt to you this turn by creatures with flying.",
            damage: "damage",
            of_it: None,
            applies: made_1_applies,
            then: None,
            text: "Prevent all damage that would be dealt to you this turn by creatures with flying.",
        },
    ],
    ..oracle::SCARECROW
};
