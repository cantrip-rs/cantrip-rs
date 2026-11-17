use cantrip_api::*;
use cantrip_mechanics::{effects, library};
use cantrip_oracle as oracle;

const REVEAL: Question = Question::new(
    "RevealMountToHand",
    "reveal it and put it into your hand?",
    "It is a Mount card. If you don't, you may put it on the bottom of your library instead",
    "Reveal it",
    "Don't",
);

const BOTTOM: Question = Question::new(
    "PutTopOnBottom",
    "put it on the bottom of your library?",
    "The card you looked at stays on top of your library if you don't",
    "Put it on the bottom",
    "Leave it on top",
);

/// "Look at the top card of your library. If it's a Mount card, you may reveal it and put it into your hand. If you don't put it
/// into your hand, you may put it on the bottom of your library." (CR 701.20a, 401.4.) "Mount" is a word of this card's text
/// (CR 612). A card that is not a Mount card, or a Mount card left unrevealed, was still looked at and may go to the bottom.
fn look_at_the_top_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mount = g.words(item.object).subtype(Subtype::Mount);
    if let Some((top, false)) = library::look_at_top_may_take(g, item, &REVEAL, |g, top| g.chars(top).has_subtype(mount))?
        && effects::ask_may(g, item.controller, item.object, Purpose::Ask(&BOTTOM))?
    {
        g.put_into_library(&[top], LibraryPlace::Bottom);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        mana_ability(
            TAP,
            add_any_color(1).only_for(ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Mount))),
            "{T}: Add one mana of any color. Spend this mana only to cast a Mount spell.",
        ),
        ability(
            cost("{3}").and(TAP),
            look_at_the_top_card,
            "{3}, {T}: Look at the top card of your library. If it's a Mount card, you may reveal it and put it into your hand. If you don't put it into your hand, you may put it on the bottom of your library.",
        ),
    ],
    ..oracle::BUCOLIC_RANCH
};
