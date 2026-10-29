use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ENTERS_OR_ATTACKS, YOUR_END_STEP};
use cantrip_mechanics::{effects, history, select};
use cantrip_oracle as oracle;

/// "A token or a land": a permanent that is a token (CR 111.1), or a land, a token land being both.
fn token_or_land(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| q.obj(id).token || q.chars(id).is(CardType::Land))
}

const TOKEN_OR_LAND: Selection =
    Selection::of(select::up_to(1), &[select::part(&(token_or_land as SelectFn), 0, "")]).called("token or land");

/// "You may sacrifice a token or a land. If you do, you gain 3 life": chosen as the ability resolves and not as a cost, and the
/// life is gained only if a permanent was actually sacrificed (CR 608.2c, 701.21a).
fn sacrifice_to_gain_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::may_sacrifice(g, item.controller, item.object, TOKEN_OR_LAND)?.is_empty() {
        effects::gain_three_life(g, item)?;
    }
    Ok(())
}

fn draw_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS_OR_ATTACKS,
            sacrifice_to_gain_life,
            "Whenever this creature enters or attacks, you may sacrifice a token or a land. If you do, you gain 3 life.",
        ),
        triggered(YOUR_END_STEP, draw_a_card, "Disappear \u{2014} At the beginning of your end step, if a permanent left the battlefield under your control this turn, draw a card.")
            .condition(history::disappear),
    ],
    ..oracle::WEST_WIND_AVATAR
};
