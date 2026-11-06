use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "When this enchantment enters, tap all red creatures." (CR 608.2, 701.26a).
fn tap_all_red_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).colors.has(q.words(item.object).color(Color::Red))
                    && q.chars(id).is(CardType::Creature)
            })
            .collect::<Vec<ObjectId>>()
    };
    g.tap_together(&objects_0);
    Ok(())
}

/// "When this enchantment enters, tap all red creatures. Red creatures don't untap during their controllers' untap steps." (CR 604.1).
fn tap_all_red_creatures_red_rule(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).colors.has(q.words(asker.source).color(Color::Red))
        && q.chars(id).is(CardType::Creature)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            tap_all_red_creatures,
            "When this enchantment enters, tap all red creatures.",
        ),
    ],
    untap_bans: &[tap_all_red_creatures_red_rule],
    ..oracle::WRATH_OF_MARIT_LAGE
};
