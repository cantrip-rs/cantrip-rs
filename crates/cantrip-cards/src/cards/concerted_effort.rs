use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// The abilities Concerted Effort hands out, in the order its text lists them: "flying ... fear, first strike, double
/// strike, landwalk, protection, trample, and vigilance." Each landwalk is an ability of its own (`ALL_LANDWALKS`: the landwalk
/// and the snow landwalk of each land type, snow landwalk, nonbasic landwalk and legendary landwalk), and protection is granted from what the creatures are protected from, beside
/// the keywords.
fn keywords() -> Vec<Keyword> {
    [Keyword::Flying, Keyword::Fear, Keyword::FirstStrike, Keyword::DoubleStrike]
        .into_iter()
        .chain(Keyword::ALL_LANDWALKS)
        .chain([Keyword::Trample, Keyword::Vigilance])
        .collect()
}

/// "At the beginning of each upkeep, creatures you control gain [each ability] until end of turn if a creature you control
/// has [it]" (CR 503.1a, 611.2a): read as the ability resolves, once for each player's upkeep, whoever's turn it is.
fn grant(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    let (mut colors, mut types, mut subtypes, mut players) = (ColorSet::NONE, TypeSet::NONE, SubtypeSet::NONE, 0u8);
    let (mut colorless, mut colored_spells, mut everything) = (false, false, false);
    let mut mana_value: Option<u32> = None;
    // Protection is whatever the creatures are protected from: colors, colorless, card types, creature types (Baneslayer
    // Angel's Demons and Dragons), a mana value or greater, colored spells, players and everything. `Chars` cannot yet
    // hold protection from multicolored or monocolored, or from type combinations (Soldier of the Pantheon, Tsabo Tavoc):
    // whoever adds such a quality must extend this copy too.
    for &id in &creatures {
        let chars = g.chars(id);
        colors = ColorSet(colors.0 | chars.protection_colors().0);
        types = TypeSet(types.0 | chars.protection_card_types().0);
        subtypes = subtypes.union(chars.protection_subtypes);
        players |= chars.protection_players;
        colorless |= chars.protection_colorless;
        colored_spells |= chars.protection_colored_spells;
        everything |= chars.protection_everything;
        // Each "N or greater" covers every greater value, so the least is all the instances cover together.
        mana_value = match (mana_value, chars.protection_mana_value) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
    }
    let protected_players: Vec<PlayerId> = g.state().seats().filter(|p| players & (1 << p.0) != 0).collect();
    instruction(g, |g| {
        effects::creatures_you_control_gain_keywords_they_have(g, item, &keywords());
        for &id in &creatures {
            if !colors.is_empty() {
                g.add_effect(item.object, id, EffectKind::Protection(colors), Duration::EndOfTurn);
            }
            if colorless {
                g.add_effect(item.object, id, EffectKind::ProtectionFromColorless, Duration::EndOfTurn);
            }
            if types != TypeSet::NONE {
                g.add_effect(item.object, id, EffectKind::ProtectionFromTypes(types), Duration::EndOfTurn);
            }
            for subtype in subtypes.iter() {
                g.add_effect(item.object, id, EffectKind::ProtectionFromSubtype(subtype), Duration::EndOfTurn);
            }
            if let Some(least) = mana_value {
                g.add_effect(item.object, id, EffectKind::ProtectionFromManaValueAtLeast(least), Duration::EndOfTurn);
            }
            if colored_spells {
                g.add_effect(item.object, id, EffectKind::ProtectionFromColoredSpells, Duration::EndOfTurn);
            }
            if everything {
                g.add_effect(item.object, id, EffectKind::ProtectionFromEverything, Duration::EndOfTurn);
            }
            for &player in &protected_players {
                g.add_effect(item.object, id, EffectKind::ProtectionFromPlayer(player), Duration::EndOfTurn);
            }
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::EACH_UPKEEP,
        grant,
        "At the beginning of each upkeep, creatures you control gain flying until end of turn if a creature you control has flying. The same is true for fear, first strike, double strike, landwalk, protection, trample, and vigilance.",
    )],
    ..oracle::CONCERTED_EFFORT
};
