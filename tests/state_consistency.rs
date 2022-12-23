use fmt::Debug;
use ft_logic_io::Action;
use ft_main_io::{FTokenAction, FTokenEvent, InitFToken};
use game_of_chance::*;
use gclient::{Error, EventListener, EventProcessor, GearApi, Result};
use gstd::prelude::*;
use primitive_types::H256;
use subxt::{
    error::{DispatchError, ModuleError, ModuleErrorData},
    Error as SubxtError,
};

const ALICE: [u8; 32] = [
    212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26, 189, 4, 169, 159, 214, 130, 44, 133, 88, 133,
    76, 205, 227, 154, 86, 132, 231, 165, 109, 162, 125,
];

fn decode<T: Decode>(payload: Vec<u8>) -> T {
    T::decode(&mut payload.as_slice()).expect("Failed to decode a reply")
}

async fn upload_code(client: &GearApi, path: &str) -> Result<H256> {
    let code_id = match client.upload_code_by_path(path).await {
        Ok((code_id, _)) => code_id.into(),
        Err(Error::Subxt(SubxtError::Runtime(DispatchError::Module(ModuleError {
            error_data:
                ModuleErrorData {
                    pallet_index: 14,
                    error: [6, 0, 0, 0],
                },
            ..
        })))) => sp_core_hashing::blake2_256(&gclient::code_from_os(path)?),
        Err(other_error) => return Err(other_error),
    };

    println!("Uploaded `{path}`.");

    Ok(code_id.into())
}

async fn upload_program_and_wait_reply<T: Decode>(
    client: &GearApi,
    listener: &mut EventListener,
    path: &str,
    payload: impl Encode,
) -> Result<([u8; 32], T)> {
    let (message_id, program_id) = common_upload_program(client, path, payload).await?;
    let (_, reply, _) = listener.reply_bytes_on(message_id.into()).await?;

    let reply = decode(reply.expect("Received an error message instead of a reply"));

    println!("Initialized `{path}`.");

    Ok((program_id, reply))
}

async fn upload_program(
    client: &GearApi,
    listener: &mut EventListener,
    path: &str,
    payload: impl Encode,
) -> Result<[u8; 32]> {
    let (message_id, program_id) = common_upload_program(client, path, payload).await?;

    assert!(listener
        .message_processed(message_id.into())
        .await?
        .succeed());
    println!("Initialized `{path}`.");

    Ok(program_id)
}

async fn common_upload_program(
    client: &GearApi,
    path: &str,
    payload: impl Encode,
) -> Result<([u8; 32], [u8; 32])> {
    let encoded_payload = payload.encode();
    let gas_limit = client
        .calculate_upload_gas(
            None,
            gclient::code_from_os(path)?,
            encoded_payload,
            0,
            true,
            None,
        )
        .await?
        .min_limit;
    let (message_id, program_id, _) = client
        .upload_program_by_path(path, gclient::bytes_now(), payload, gas_limit, 0)
        .await?;

    Ok((message_id.into(), program_id.into()))
}

async fn send_message_with_custom_limit<T: Decode>(
    client: &GearApi,
    listener: &mut EventListener,
    destination: [u8; 32],
    payload: impl Encode + Debug,
    modify_gas_limit: fn(u64) -> u64,
) -> Result<Result<T, String>> {
    let encoded_payload = payload.encode();
    let destination = destination.into();

    let gas_limit = client
        .calculate_handle_gas(None, destination, encoded_payload, 0, true, None)
        .await?
        .min_limit;
    let modified_gas_limit = modify_gas_limit(gas_limit);

    println!("Sending payload: `{payload:?}`.");
    println!("Calculated gas limit: {gas_limit}.");
    println!("Modified gas limit: {modified_gas_limit}.");

    let (message_id, _) = client
        .send_message(destination, payload, modified_gas_limit, 0)
        .await?;

    println!("Sending completed.");

    let (_, reply, _) = listener.reply_bytes_on(message_id).await?;

    Ok(reply.map(|reply| decode(reply)))
}

async fn send_message<T: Decode>(
    client: &GearApi,
    listener: &mut EventListener,
    destination: [u8; 32],
    payload: impl Encode + Debug,
) -> Result<T> {
    Ok(
        send_message_with_custom_limit(client, listener, destination, payload, |gas| gas * 2)
            .await?
            .expect("Received an error message instead of a reply"),
    )
}

async fn send_message_for_goc(
    client: &GearApi,
    listener: &mut EventListener,
    destination: [u8; 32],
    payload: impl Encode + Debug,
) -> Result<Result<GOCEvent, GOCError>> {
    send_message(client, listener, destination, payload).await
}

async fn send_message_with_insufficient_gas(
    client: &GearApi,
    listener: &mut EventListener,
    destination: [u8; 32],
    payload: impl Encode + Debug,
) -> Result<String> {
    Ok(
        send_message_with_custom_limit::<()>(client, listener, destination, payload, |gas| {
            gas - gas / 100
        })
        .await?
        .expect_err("Received a reply instead of an error message"),
    )
}

#[tokio::test]
#[ignore]
async fn state_consistency() -> Result<()> {
    let client = GearApi::dev().await?;
    let mut listener = client.subscribe().await?;

    let storage_code_hash = upload_code(&client, "target/ft_storage.wasm").await?;
    let ft_logic_code_hash = upload_code(&client, "target/ft_logic.wasm").await?;

    let ft_actor_id = upload_program(
        &client,
        &mut listener,
        "target/ft_main.wasm",
        InitFToken {
            storage_code_hash,
            ft_logic_code_hash,
        },
    )
    .await?;

    let (goc_actor_id, reply) = upload_program_and_wait_reply::<Result<(), GOCError>>(
        &client,
        &mut listener,
        "target/wasm32-unknown-unknown/debug/game_of_chance.opt.wasm",
        GOCInit {
            admin: ALICE.into(),
        },
    )
    .await?;
    assert_eq!(reply, Ok(()));

    let amount = 12345;

    assert!(
        FTokenEvent::Ok
            == send_message(
                &client,
                &mut listener,
                ft_actor_id,
                FTokenAction::Message {
                    transaction_id: 0,
                    payload: Action::Mint {
                        recipient: ALICE.into(),
                        amount,
                    }
                    .encode(),
                },
            )
            .await?
    );
    assert!(
        FTokenEvent::Ok
            == send_message(
                &client,
                &mut listener,
                ft_actor_id,
                FTokenAction::Message {
                    transaction_id: 1,
                    payload: Action::Approve {
                        approved_account: goc_actor_id.into(),
                        amount,
                    }
                    .encode(),
                },
            )
            .await?
    );

    println!(
        "{:?}",
        send_message::<Result<GOCEvent, GOCError>>(
            &client,
            &mut listener,
            goc_actor_id,
            GOCAction::Start {
                duration: 15000,
                participation_cost: 10000,
                ft_actor_id: Some(ft_actor_id.into())
            }
        )
        .await?
    );

    let mut payload = GOCAction::Enter;

    println!(
        "{}",
        send_message_with_insufficient_gas(&client, &mut listener, goc_actor_id, payload).await?
    );
    // TODO: add an equality check for reply after after introducing an
    // equivalent to the gtest's `System::block_timestamp()` function
    // (https://github.com/gear-tech/gear/issues/1962#issuecomment-1354489919).
    println!(
        "{:?}",
        send_message_for_goc(&client, &mut listener, goc_actor_id, payload).await?
    );

    payload = GOCAction::PickWinner;

    println!(
        "{}",
        send_message_with_insufficient_gas(&client, &mut listener, goc_actor_id, payload).await?
    );
    assert_eq!(
        Ok(GOCEvent::Winner(ALICE.into())),
        send_message_for_goc(&client, &mut listener, goc_actor_id, payload).await?
    );

    Ok(())
}
