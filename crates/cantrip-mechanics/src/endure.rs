//! Endure N (CR 701.63): "That permanent's controller creates an N/N white Spirit creature token unless they put N +1/+1
//! counters on that permanent." Anafenza, Unyielding Lineage, Fortress Kin-Guard, Descendant of Storms, Krumar Initiate and
//! Warden of the Grove print it.

use cantrip_api::*;

const ENDURE: Question = Question::new("PutCounters", "put the +1/+1 counters on it instead of creating a Spirit token?", "", "Put counters", "Create a token");

/// The white Spirit creature token endure makes (CR 701.63a): the generic definition, its size N/N given as it is made
/// (`endure`). A card that endures lists it in its `EXTRA`, as do Abzan Monument and Severance Priest (their X/Xs), Great Arashin City and Revival
/// of the Ancestors (their 1/1s), which make the same white Spirit.
pub static SPIRIT: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(0),
    ..token!(Spirit)
};

/// `permanent` endures `n` (CR 701.63a): its controller chooses as this resolves (rulings 2025-04-04) between putting `n` +1/+1
/// counters on it and creating an `n`/`n` white Spirit token, N/N as it is made (CR 111.4, 707.2). A permanent that has left
/// the battlefield or phased out, or that counters can't be put on (Solemnity, Blossombind; CR 122.6, 101.2), cannot take them:
/// the controller is not asked and the token is made (CR 701.63a; its ruling). Endure 0 does nothing (CR 701.63b).
pub fn endure(g: &mut dyn Game, permanent: ObjectId, source: ObjectId, n: i32) -> Result<(), Suspend> {
    if n == 0 {
        return Ok(());
    }
    let controller = g.controller(permanent);
    let can_take_counters = g.is_on_battlefield(permanent) && !g.counters_banned(permanent, controller, CounterKind::PlusOne);
    if can_take_counters && crate::effects::ask_may(g, controller, source, Purpose::Ask(&ENDURE))? {
        g.add_counters_by(controller, permanent, CounterKind::PlusOne, n);
    } else {
        g.create_token_copies_of(Copiable::of(g.card_id(&SPIRIT)).sized(n, n), controller, 1)?;
    }
    Ok(())
}
