use cantrip_api::*;
use cantrip_mechanics::{statics, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// The 8/8 blue Leviathan creature token with hexproof.
pub static LEVIATHAN: CardDef = CardDef {
    name: "Leviathan Token",
    token_name: Some(&[Subtype::Leviathan]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Leviathan]),
    power: Some(8),
    toughness: Some(8),
    keywords: Keywords::of(&[Keyword::Hexproof]),
    text: "Hexproof",
    ..CardDef::BLANK
};

fn make_leviathan(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &LEVIATHAN, 1)?;
    Ok(())
}

/// "If you've activated a loyalty ability this turn" Check any loyalty activation this turn, both on triggering
/// and resolution (CR 603.4).
fn activated_a_loyalty_ability(q: &dyn Query, asker: &Asker) -> bool {
    q.history()
        .select(EventQuery::kind(EventMask::ABILITY_ACTIVATED).player(asker.controller).this_turn())
        .any(|r| {
            matches!(r.event, Event::AbilityActivated { ability, .. }
                if q.activated_ability(ability).is_some_and(|def| def.is_loyalty_ability()))
        })
}

/// "Untap target attacking creature. It can't be blocked this turn." (CR 611.2c)
fn untap_and_unblockable(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.untap(creature);
        g.add_effect(
            item.object,
            creature,
            EffectKind::AddKeywords(Keywords::of(&[Keyword::CantBeBlocked])),
            Duration::EndOfTurn,
        );
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_ATTACK,
        untap_and_unblockable,
        "Whenever you attack, if you've activated a loyalty ability this turn, untap target attacking creature. It can't be blocked this turn.",
    )
    .condition(activated_a_loyalty_ability)
    .targets(&[targets::ATTACKING_CREATURE])],
    delayed: &[ability(
        loyalty(-8),
        make_leviathan,
        "\u{2212}8: Create an 8/8 blue Leviathan creature token with hexproof.",
    )],
    statics: &[static_ability(
        Layer::Ability,
        statics::your_planeswalkers,
        statics::grant_delayed_ability,
        "Planeswalkers you control have \"\u{2212}8: Create an 8/8 blue Leviathan creature token with hexproof.\"",
    )],
    ..oracle::KIORA_OF_SALT_AND_SAND
};

pub static EXTRA: &[&CardDef] = &[&LEVIATHAN];
