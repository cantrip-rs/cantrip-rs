use cantrip_api::*;
use cantrip_mechanics::{targets, teamwork, tokens};
use cantrip_oracle as oracle;

/// The 6/5 blue Leviathan creature token with hexproof.
pub static LEVIATHAN: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(6),
    toughness: Some(5),
    keywords: Keywords::of(&[Keyword::Hexproof]),
    ..token!(Leviathan)
};

/// "Target player creates a 6/5 blue Leviathan creature token with hexproof": the player targeted makes it and controls it, if
/// they are still a legal target (CR 608.2b). The mode's target is the first slot of its own (CR 700.2c).
fn leviathan(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    tokens::create(g, player, &LEVIATHAN, 1)?;
    Ok(())
}

/// "Return one or two target nonland permanents to their owners' hands": those still legal go together (CR 608.2b, 400.7).
fn return_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.return_to_hand(&targets::legal_objects(g, item));
    Ok(())
}

/// "Choose one. If this spell was cast using teamwork, choose both instead." (CR 700.2, 702.194): with the teamwork cost paid,
/// both modes, each with its own targets (CR 700.2c), done in printed order (CR 700.2d).
static MODAL: ModalDef = modes(&[
    mode(leviathan, "Target player creates a 6/5 blue Leviathan creature token with hexproof.").targets(&[targets::PLAYER]),
    mode(return_them, "Return one or two target nonland permanents to their owners' hands.")
        .targets(&[targets::NONLAND_PERMANENT.between(1, 2)]),
])
.when_paid(2, 2);

/// "Teamwork 4": announced before the modes, since how many are chosen is defined by it (CR 702.194a, 700.2a).
pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    optional_cost: Some(teamwork::optional(4)),
    labels: &[
        "Cast without teamwork (choose one)",
        "Teamwork 4: Tap any number of creatures you control with total power 4 or more (choose both)",
    ],
    ..oracle::ATLANTIS_ATTACKS
};

pub static EXTRA: &[&CardDef] = &[&LEVIATHAN];
