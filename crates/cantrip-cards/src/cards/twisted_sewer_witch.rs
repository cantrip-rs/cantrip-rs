use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// The 1/1 black Rat creature token with "This creature can't block." (CR 111.1, 509.1b), worded as this card words it: the same
/// token as `tokens::RAT_CANT_BLOCK`, whose own text says "This token".
static RAT: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::CantBlock]),
    text: "This creature can't block.",
    ..token!(Rat)
};

/// "Create a 1/1 black Rat creature token with 'This creature can't block.' Then for each Rat you control, create a Wicked Role
/// token attached to that Rat." (CR 111.10q, 608.2c, 303.7.) The Rats are counted after the token is made, so it counts, and
/// so does every other Rat you control, token or not. The Roles are all one instruction, created as one event (CR 101.4, 603.2c), each attached to its own Rat; a Rat that
/// is not a creature cannot be enchanted by a Role (CR 303.4i) and gets none.
fn rat_and_roles(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    tokens::create(g, you, &RAT, 1)?;
    g.end_sentence();
    let rat = g.words(item.object).subtype(Subtype::Rat);
    let rats: Vec<ObjectId> = g.permanents_of(you).into_iter().filter(|&id| g.chars(id).has_subtype(rat)).collect();
    tokens::create_roles(g, &tokens::WICKED_ROLE, you, &rats)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        rat_and_roles,
        "When this creature enters, create a 1/1 black Rat creature token with \"This creature can't block.\" Then for each Rat you control, create a Wicked Role token attached to that Rat.",
    )],
    ..oracle::TWISTED_SEWER_WITCH
};

pub static EXTRA: &[&CardDef] = &[&RAT, &tokens::WICKED_ROLE];
