use gstd::{prelude::*, ActorId};

/// Initializes the lottery contract.
///
/// # Requirements
/// - `admin` mustn't be [`ActorId::zero()`].
#[derive(Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, TypeInfo)]
pub struct LotteryInit {
    /// [`ActorId`] of an administrator that'll have the rights to
    /// [start a lottery](LotteryAction::Start) and
    /// [pick a winner](LotteryAction::PickWinner).
    pub admin: ActorId,
}

/// Sends a contract info about what it should do.
#[derive(Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, TypeInfo)]
pub enum LotteryAction {
    /// Starts a lottery round and allows to participate in it.
    ///
    /// # Requirements
    /// - [`msg::source()`] must be an administrator.
    /// - The previous lottery round must be over.
    /// - `ft_actor_id` mustn't be [`ActorId::zero()`].
    ///
    /// On success, returns [`LotteryEvent::Started`].
    ///
    /// [`msg::source()`]: gstd::msg::source
    Start {
        /// The duration (in milliseconds) of the players entry stage.
        ///
        /// After that, no one will be able to enter a lottery and a winner
        /// should be picked.
        duration: u64,
        /// The price of participation in a new lottery round.
        participation_cost: u128,
        /// A currency (or a FT contract [`ActorId`]) of a new lottery round.
        ///
        /// Determines fungible tokens in which a prize fund and a participation
        /// cost will be collected. [`None`] means that the native value will be
        /// used instead of fungible tokens.
        ft_actor_id: Option<ActorId>,
    },

    /// Randomly picks a winner from current lottery round participants
    /// (players) and sends a prize fund to it.
    ///
    /// The randomness of a winner pick depends on [`exec::block_timestamp()`].
    /// Not the best source of entropy, but, in theory, it's impossible to
    /// exactly predict a winner if the time of an execution of this action is
    /// unknown.
    ///
    /// If no one participated in the round, then a winner will be
    /// [`ActorId::zero()`].
    ///
    /// # Requirements
    /// - [`msg::source()`] must be an administrator.
    /// - The players entry stage must be over.
    /// - A winner mustn't already be picked.
    ///
    /// On success, returns [`LotteryEvent::Winner`].
    ///
    /// [`exec::block_timestamp()`]: gstd::exec::block_timestamp
    /// [`msg::source()`]: gstd::msg::source
    PickWinner,

    /// Pays a participation cost on behalf of [`msg::source()`] and adds it to
    /// lottery participants (players).
    ///
    /// A participation cost and its currency can be queried by
    /// [`LotteryStateQuery::State`].
    ///
    /// # Requirements
    /// - The players entry stage mustn't be over.
    /// - [`msg::source()`] mustn't already participate.
    /// - [`msg::source()`] must have enough currency to pay a participation
    /// cost.
    /// - If the current lottery round currency is the native value
    /// (`ft_actor_id` is [`None`]), [`msg::source()`] must send this action
    /// with the amount of the value exactly equal to a participation cost.
    ///
    /// On success, returns [`LotteryEvent::PlayerAdded`].
    ///
    /// [`msg::source()`]: gstd::msg::source
    Enter,
}

/// A result of processed [`LotteryAction`].
#[derive(Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, TypeInfo)]
pub enum LotteryEvent {
    /// Should be returned from [`LotteryAction::Start`].
    Started {
        /// The end time (in milliseconds) of the players entry stage.
        ///
        /// After that, a lottery administrator can pick a winner.
        ending: u64,
        /// See the documentation of [`LotteryAction::Start`].
        participation_cost: u128,
        /// See the documentation of [`LotteryAction::Start`].
        ft_actor_id: Option<ActorId>,
    },
    /// Should be returned from [`LotteryAction::PickWinner`].
    Winner(ActorId),
    /// Should be returned from [`LotteryAction::Enter`].
    PlayerAdded(ActorId),
}

/// Queries a contract state.
#[derive(Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, TypeInfo)]
pub enum LotteryStateQuery {
    /// Queries a lottery state.
    ///
    /// Returns [`LotteryStateReply::State`].
    State,
}

/// A reply to queried [`LotteryStateQuery`].
#[derive(Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Clone, TypeInfo)]
pub enum LotteryStateReply {
    /// Should be returned from [`LotteryStateQuery::State`].
    State {
        /// The start time (in milliseconds) of the current lottery round and
        /// the players entry stage.
        ///
        /// If it equals 0, a winner has picked and the current round is over.
        started: u64,
        /// See the documentation of [`LotteryEvent::Started`].
        ending: u64,
        /// Participants of the current lottery round.
        players: BTreeSet<ActorId>,
        /// The current lottery round prize fund.
        ///
        /// It's calculated by multiplying `participation_cost` and a number of
        /// `players`.
        prize_fund: u128,
        /// See the documentation of [`LotteryAction::Start`].
        participation_cost: u128,
        /// The winner of the last lottery round.
        last_winner: ActorId,
        /// A currency (or a FT contract [`ActorId`]) of the current lottery
        /// round.
        ///
        /// See the documentation of [`LotteryAction::Start`].
        ft_actor_id: Option<ActorId>,
    },
}
