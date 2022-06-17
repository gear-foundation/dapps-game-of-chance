#![no_std]

#[cfg(test)]
mod simple_tests;

#[cfg(test)]
mod panic_tests;

#[cfg(test)]
mod token_tests;

use codec::{Decode, Encode};
use ft_io::*;
use gstd::{debug, exec, msg, prelude::*, ActorId};
use lt_io::*;
use scale_info::TypeInfo;

#[derive(Debug, Default, Encode, Decode, TypeInfo)]
struct Lottery {
    lottery_state: LotteryState,
    lottery_owner: ActorId,
    token_address: Option<ActorId>,
    players: BTreeMap<u32, Player>,
    lottery_history: BTreeMap<u32, ActorId>,
    lottery_id: u32,
}

static mut LOTTERY: Option<Lottery> = None;

impl Lottery {
    // checks that lottery has started and lottery time has not expired
    fn lottery_is_on(&mut self) -> bool {
        self.lottery_state.lottery_started
            && (self.lottery_state.lottery_start_time + self.lottery_state.lottery_duration)
                > exec::block_timestamp()
    }

    /// Launches a lottery
    /// Requirements:
    /// * Only owner can launch lottery
    /// * Lottery must not have been launched earlier
    /// Arguments:
    /// * `duration`: lottery duration in milliseconds
    /// * `token_address`: address of Fungible Token contract
    fn start_lottery(
        &mut self,
        duration: u64,
        token_address: Option<ActorId>,
        participation_cost: u128,
        prize_fund: u128,
    ) {
        if msg::source() == self.lottery_owner && !self.lottery_is_on() {
            self.lottery_state.lottery_started = true;
            self.lottery_state.lottery_start_time = exec::block_timestamp();
            self.lottery_state.lottery_duration = duration;
            self.lottery_state.participation_cost = participation_cost;
            self.lottery_state.prize_fund = prize_fund;
            self.lottery_state.lottery_owner = self.lottery_owner;
            self.token_address = token_address;
            self.lottery_id = self.lottery_id.saturating_add(1);
        } else {
            panic!(
                "start_lottery(): Lottery on: {}  Owner message: {}  source(): {:?}  owner: {:?}",
                self.lottery_is_on(),
                msg::source() == self.lottery_owner,
                msg::source(),
                self.lottery_owner
            );
        }
    }

    // checks that the player is already participating in the lottery
    fn player_exists(&mut self) -> bool {
        self.players
            .values()
            .any(|player| player.player_id == msg::source())
    }

    /// Transfers `amount` tokens from `sender` account to `recipient` account.
    /// Arguments:
    /// * `from`: sender account
    /// * `to`: recipient account
    /// * `amount`: amount of tokens
    async fn transfer_tokens(
        &mut self,
        token_address: &ActorId,
        from: &ActorId,
        to: &ActorId,
        amount_tokens: u128,
    ) {
        msg::send_and_wait_for_reply::<FTEvent, _>(
            *token_address,
            FTAction::Transfer {
                from: *from,
                to: *to,
                amount: amount_tokens,
            },
            0,
        )
        .unwrap()
        .await
        .expect("Error in transfer");
    }

    /// Called by a player in order to participate in lottery
    /// Requirements:
    /// * Lottery must be on
    /// * Contribution must be greater than zero
    /// * The player cannot enter the lottery more than once
    /// Arguments:
    /// * `amount`: value or tokens to participate in the lottery
    async fn enter(&mut self, amount: u128) {
        if self.lottery_is_on()
            && !self.player_exists()
            && amount == self.lottery_state.participation_cost
        {
            if let Some(token_address) = self.token_address {
                self.transfer_tokens(&token_address, &msg::source(), &exec::program_id(), amount)
                    .await;

                debug!("Add in Fungible Token: {}", amount);
            }

            let player = Player {
                player_id: msg::source(),
                balance: amount,
            };

            let player_index = self.players.len() as u32;
            self.players.insert(player_index, player);
            msg::reply(LtEvent::PlayerAdded(player_index), 0).unwrap();
        } else {
            panic!(
                "enter(): Lottery on: {}  player exists: {} amount: {}",
                self.lottery_is_on(),
                self.player_exists(),
                amount
            );
        }
    }

    // Random number generation from block timestamp
    fn get_random_number(&mut self) -> u32 {
        let timestamp: u64 = exec::block_timestamp();
        let code_hash = sp_core_hashing::blake2_256(&timestamp.to_le_bytes());
        u32::from_le_bytes([code_hash[0], code_hash[1], code_hash[2], code_hash[3]])
    }

    /// Lottery winner calculation
    /// Requirements:
    /// * Only owner can pick the winner
    /// * Lottery has started and lottery time is expired
    /// * List of players must not be empty
    async fn pick_winner(&mut self) {
        if msg::source() == self.lottery_owner && !self.players.is_empty() {
            let index = (self.get_random_number() % (self.players.len() as u32)) as usize;
            let win_player_index = *self.players.keys().nth(index).expect("Player not found");
            let player = self.players[&win_player_index];

            if let Some(token_address) = self.token_address {
                debug!("Transfer tokens to the winner");
                self.transfer_tokens(
                    &token_address,
                    &exec::program_id(),
                    &player.player_id,
                    self.lottery_state.prize_fund,
                )
                .await;
            } else {
                msg::send_bytes(player.player_id, b"Winner", self.lottery_state.prize_fund)
                    .unwrap();
            }

            self.lottery_history
                .insert(self.lottery_id, player.player_id);
            msg::reply(LtEvent::Winner(win_player_index), 0).unwrap();

            debug!(
                "Winner: {} token_address(): {:?}",
                index, self.token_address
            );

            self.token_address = None;
            self.lottery_state = LotteryState::default();
            self.players = BTreeMap::new();
        } else {
            panic!(
                "pick_winner(): Owner message: {}  lottery_duration: {}  players.is_empty(): {}",
                msg::source() == self.lottery_owner,
                self.lottery_state.lottery_start_time + self.lottery_state.lottery_duration
                    > exec::block_timestamp(),
                self.players.is_empty()
            );
        }
    }
}

#[gstd::async_main]
async fn main() {
    let lottery = unsafe { LOTTERY.get_or_insert(Lottery::default()) };
    let action: LtAction = msg::load().expect("Could not load Action");

    match action {
        LtAction::Enter(amount) => {
            lottery.enter(amount).await;
        }

        LtAction::StartLottery {
            duration,
            token_address,
            participation_cost,
            prize_fund,
        } => {
            lottery.start_lottery(duration, token_address, participation_cost, prize_fund);
        }

        LtAction::LotteryState => {
            msg::reply(LtEvent::LotteryState(lottery.lottery_state.clone()), 0).unwrap();
            debug!("LotteryState: {:?}", lottery.lottery_state);
        }

        LtAction::PickWinner => {
            lottery.pick_winner().await;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn init() {
    let mut lottery = Lottery {
        lottery_owner: msg::source(),
        ..Default::default()
    };

    lottery.lottery_state.lottery_owner = lottery.lottery_owner;
    LOTTERY = Some(lottery);
}

#[no_mangle]
pub unsafe extern "C" fn meta_state() -> *mut [i32; 2] {
    let query: LtState = msg::load().expect("failed to decode input argument");
    let lottery = LOTTERY.get_or_insert(Lottery::default());

    let encoded = match query {
        LtState::GetPlayers => LtStateReply::Players(lottery.players.clone()).encode(),
        LtState::GetWinners => LtStateReply::Winners(lottery.lottery_history.clone()).encode(),
        LtState::LotteryState => LtStateReply::LotteryState(lottery.lottery_state.clone()).encode(),

        LtState::BalanceOf(index) => {
            if let Some(player) = lottery.players.get(&index) {
                LtStateReply::Balance(player.balance).encode()
            } else {
                LtStateReply::Balance(0).encode()
            }
        }
    };

    gstd::util::to_leak_ptr(encoded)
}

gstd::metadata! {
    title: "Lottery",
    handle:
        input: LtAction,
        output: LtEvent,
    state:
        input: LtState,
        output: LtStateReply,
}
