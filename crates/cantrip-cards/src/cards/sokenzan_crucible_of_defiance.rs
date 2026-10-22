use cantrip_api::*;
use cantrip_mechanics::{channel, haste_until_end_of_turn, tokens};
use cantrip_oracle as oracle;

/// The 1/1 colorless Spirit creature token this card makes (CR 111.1): no abilities of its own.
static SPIRIT: CardDef = CardDef {
    name: "Spirit Token",
    token_name: Some(&[Subtype::Spirit]),
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Spirit]),
    power: Some(1),
    toughness: Some(1),
    shares_name: true,
    ..CardDef::BLANK
};

/// "Create two 1/1 colorless Spirit creature tokens. They gain haste until end of turn" (CR 111.1, 611.2): both tokens,
/// created together, gain haste from this ability's source.
fn two_hasty_spirits(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let spirits = tokens::create(g, item.controller, &SPIRIT, 2)?;
    g.end_sentence();
    instruction(g, |g| {
        for spirit in spirits {
            haste_until_end_of_turn(g, item.object, spirit);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::R), "{T}: Add {R}."),
        ability(cost("{3}{R}").and(DISCARD_THIS), two_hasty_spirits, "Channel \u{2014} {3}{R}, Discard this card: Create two 1/1 colorless Spirit creature tokens. They gain haste until end of turn. This ability costs {1} less to activate for each legendary creature you control.")
            .from(ZoneKind::Hand),
    ],
    own_cost_changes: &[channel::cheaper_per_legendary_creature],
    ..oracle::SOKENZAN_CRUCIBLE_OF_DEFIANCE
};

pub static EXTRA: &[&CardDef] = &[&SPIRIT];
