use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Investigate." (CR 701.16a)
fn investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)
}

/// A Clue (CR 111.10f, 701.21a): any Clue artifact, a token or not (the ruling), of the permanents its controller can sacrifice.
fn clue(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(Subtype::Clue)
}

/// "You draw X cards and gain X life" (CR 121.2, 119.3, 608.2c): X is the value announced as the ability was activated; one
/// sentence, so the cards are drawn, then the life gained.
fn draw_x_cards_and_gain_x_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, item.x)?;
    effects::gain_life(g, item, item.x as i32)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, investigate, "When Alquist Proft enters, investigate."),
        ability(
            cost("{X}{W}{U}{U}, {T}").and(sacrifice(1, clue)),
            draw_x_cards_and_gain_x_life,
            "{X}{W}{U}{U}, {T}, Sacrifice a Clue: You draw X cards and gain X life.",
        ),
    ],
    ..oracle::ALQUIST_PROFT_MASTER_SLEUTH
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
