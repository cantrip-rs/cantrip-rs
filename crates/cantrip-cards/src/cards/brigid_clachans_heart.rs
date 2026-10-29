use cantrip_api::*;
use cantrip_mechanics::tokens::{self, KITHKIN};
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&KITHKIN];

/// "{T}: Add X {G} or X {W}, where X is the number of other creatures you control." (CR 605.1a, 106.5): a mana ability, so
/// the color is chosen as it is activated (CR 106.1b) and the creatures are counted as it resolves, possibly none.
fn add_for_each_other_creature(g: &mut dyn Game, asker: &Asker, color: Option<Color>) -> Result<(), Suspend> {
    let others = g.permanents_of(asker.controller).into_iter().filter(|&id| id != asker.source && g.chars(id).is(CardType::Creature)).count();
    if let Some(color) = color
        && others > 0
    {
        g.add_mana(asker.controller, color.mana(), others as u32);
    }
    Ok(())
}

/// "At the beginning of your first main phase, you may pay {G}. If you do, transform Brigid."
fn pay_green(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_transform(g, item, mana("{G}"))
}

/// "At the beginning of your first main phase, you may pay {W}. If you do, transform Brigid."
fn pay_white(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_transform(g, item, mana("{W}"))
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add_one_of(&[Color::Green, Color::White]).by(add_for_each_other_creature), "{T}: Add X {G} or X {W}, where X is the number of other creatures you control."),
        triggered(triggers::YOUR_FIRST_MAIN_PHASE, pay_white, "At the beginning of your first main phase, you may pay {W}. If you do, transform Brigid."),
    ],
    ..oracle::BRIGID_DOUNS_MIND
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS_OR_TRANSFORMS, tokens::create_a_kithkin, "Whenever this creature enters or transforms into Brigid, Clachan's Heart, create a 1/1 green and white Kithkin creature token."),
        triggered(triggers::YOUR_FIRST_MAIN_PHASE, pay_green, "At the beginning of your first main phase, you may pay {G}. If you do, transform Brigid."),
    ],
    back: Some(&BACK),
    ..oracle::BRIGID_CLACHANS_HEART
};
