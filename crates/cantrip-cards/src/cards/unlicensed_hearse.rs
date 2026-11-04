use cantrip_api::*;
use cantrip_mechanics::{crew::crew, targets};
use cantrip_oracle as oracle;

/// "Exile up to two target cards from a single graveyard." The cards still in that graveyard are exiled together (CR
/// 608.2b: a target that became illegal is left alone), each exiled with this Vehicle (CR 607.2a), where its power and
/// toughness count them.
fn exile_cards(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards: Vec<ObjectId> = (0..2).filter_map(|slot| targets::object(g, item, slot)).collect();
    g.exile_with(&cards, g.linked(item), false);
    Ok(())
}

/// "Unlicensed Hearse's power and toughness are each equal to the number of cards exiled with it" (CR 604.3, 607.2a): the
/// cards in exile whose link points back at it, a characteristic-defining ability in every zone. A Vehicle that left and
/// came back is a new object (CR 400.7) with none.
fn exiled_with_it(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let n = q.exiled_with(q.linked_to(source)).len();
    chars.power = Some(n as i16);
    chars.toughness = Some(n as i16);
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, exile_cards, "{T}: Exile up to two target cards from a single graveyard.").targets(targets::UP_TO_TWO_CARDS_FROM_A_SINGLE_GRAVEYARD),
        crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    statics: &[characteristic_defining(Layer::PtDefine, exiled_with_it, "Unlicensed Hearse's power and toughness are each equal to the number of cards exiled with it.").lost_pt(0, 0)],
    ..oracle::UNLICENSED_HEARSE
};
