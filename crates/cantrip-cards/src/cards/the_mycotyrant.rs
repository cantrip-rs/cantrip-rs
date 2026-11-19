use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::{history, lands, tokens};
use cantrip_oracle as oracle;

/// "The Mycotyrant's power and toughness are each equal to the number of creatures you control that are Fungi and/or Saprolings"
/// (CR 604.3, 613.4a): characteristic-defining, so it works in every zone, and it counts the creatures as the earlier layers made
/// them, itself among them (it is a Fungus). The two subtypes are words of its own text (CR 612).
fn power_and_toughness_are_fungi_and_saprolings_you_control(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let words = q.words(source);
    let (fungus, saproling) = (words.subtype(Subtype::Fungus), words.subtype(Subtype::Saproling));
    let n = lands::count_you_control(q, source, &|c| c.is(CardType::Creature) && (c.has_subtype(fungus) || c.has_subtype(saproling)));
    chars.power = Some(n);
    chars.toughness = Some(n);
}

/// "Create X 1/1 black Fungus creature tokens with 'This token can't block,' where X is the number of times you descended this
/// turn" (CR 700.11, 111.1): counted as the trigger resolves, all made together; none for no descent.
fn create_a_fungus_for_each_time_you_descended(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let n = history::times_descended_this_turn(g, item.controller) as u32;
    tokens::create(g, item.controller, &tokens::FUNGUS_CANT_BLOCK, n)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        create_a_fungus_for_each_time_you_descended,
        "At the beginning of your end step, create X 1/1 black Fungus creature tokens with \"This token can't block,\" where X is the number of times you descended this turn.",
    )],
    statics: &[characteristic_defining(
        Layer::PtDefine,
        power_and_toughness_are_fungi_and_saprolings_you_control,
        "The Mycotyrant's power and toughness are each equal to the number of creatures you control that are Fungi and/or Saprolings.",
    )
    .lost_pt(0, 0)],
    ..oracle::THE_MYCOTYRANT
};

pub static EXTRA: &[&CardDef] = &[&tokens::FUNGUS_CANT_BLOCK];
