use cantrip_api::*;
use cantrip_mechanics::library::{self, Rest};
use cantrip_oracle as oracle;

/// "Look at the top three cards of your library. You may reveal a Hero card from among them and put it into your hand. Put
/// the rest on the bottom of your library in any order." (CR 401.2, 401.4, 701.20): "Hero" read through this land's text
/// (CR 612).
fn look_for_a_hero(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hero = g.words(item.object).subtype(Subtype::Hero);
    library::look_at_top(g, item.controller, item.object, 3, (0, 1), true, |c| c.has_subtype(hero), Rest::BottomChosen)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        mana_ability(
            TAP,
            add_any_color(1).only_for(ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Hero).or_abilities())),
            "{T}: Add one mana of any color. Spend this mana only to cast a Hero spell or to activate an ability of a Hero source.",
        ),
        ability(
            cost("{4}, {T}"),
            look_for_a_hero,
            "{4}, {T}: Look at the top three cards of your library. You may reveal a Hero card from among them and put it into your hand. Put the rest on the bottom of your library in any order.",
        ),
    ],
    ..oracle::AVENGERS_TOWER
};
