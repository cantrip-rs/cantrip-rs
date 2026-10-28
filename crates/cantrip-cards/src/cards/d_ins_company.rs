use cantrip_api::*;
use cantrip_mechanics::library::{look_at_top, Rest};
use cantrip_mechanics::lands;
use cantrip_mechanics::statics::itself_while_you_control_another;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

fn lifelink(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Lifelink);
}

/// "When this creature enters, look at the top four cards of your library. You may reveal a Dwarf or Equipment card from among
/// them and put it into your hand. Put the rest on the bottom of your library in a random order." The creature type is read
/// through this card's words (CR 612); Equipment is an artifact type, which no text-changing effect touches. With fewer
/// than four cards in the library it looks at what there is (CR 609.3), and with none to take it still shows the cards.
fn look(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let dwarf = g.words(item.object).subtype(Subtype::Dwarf);
    look_at_top(
        g,
        item.controller,
        item.object,
        4,
        (0, 1),
        true,
        |c| c.has_subtype(dwarf) || c.has_subtype(Subtype::Equipment),
        Rest::BottomRandom,
    )?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        look,
        "When this creature enters, look at the top four cards of your library. You may reveal a Dwarf or Equipment card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
    )],
    statics: &[static_ability(Layer::Ability, itself_while_you_control_another::<{ lands::sub(Subtype::Dwarf) }>, lifelink, "This creature has lifelink as long as you control another Dwarf.")],
    ..oracle::D_INS_COMPANY
};
