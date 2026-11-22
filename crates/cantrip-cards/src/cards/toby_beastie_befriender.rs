use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{statics, tokens};
use cantrip_oracle as oracle;

/// The 4/4 white Beast creature token with "This token can't attack or block alone" (CR 111.3, 111.4). "Alone" (CR 506.5) is one
/// notion for both sides: a declaration with this token in it has at least two creatures (CR 508.1c, 509.1b).
static BEAST: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(4),
    combat_groups: &[
        CombatGroupRule { role: CombatRole::Attacker, about: Some(filters::this_permanent), at_least: 2, at_most: u32::MAX },
        CombatGroupRule { role: CombatRole::Blocker, about: Some(filters::this_permanent), at_least: 2, at_most: u32::MAX },
    ],
    text: "This token can't attack or block alone.",
    ..token!(Beast)
};

/// "When Toby enters, create a 4/4 white Beast creature token with 'This token can't attack or block alone.'"
fn create_the_beast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &BEAST, 1)?;
    Ok(())
}

/// "As long as you control four or more creature tokens, creature tokens you control have flying." (CR 604.1, 613.1f):
/// the count is of creatures as the earlier layers made them, so a token that stops being a creature is not counted, and it
/// is asked again whenever characteristics are computed.
fn four_creature_tokens_you_control_fly(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    let you = q.controller(source);
    statics::your_creature_tokens(q, source, affected, so_far)
        && q.state().battlefield().iter().copied().filter(|&id| q.state().obj(id).token && q.controller(id) == you && q.chars(id).is(CardType::Creature)).count() >= 4
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        create_the_beast,
        "When Toby enters, create a 4/4 white Beast creature token with \"This token can't attack or block alone.\"",
    )],
    statics: &[static_ability(
        Layer::Ability,
        four_creature_tokens_you_control_fly,
        statics::have_flying,
        "As long as you control four or more creature tokens, creature tokens you control have flying.",
    )],
    ..oracle::TOBY_BEASTIE_BEFRIENDER
};

pub static EXTRA: &[&CardDef] = &[&BEAST];
