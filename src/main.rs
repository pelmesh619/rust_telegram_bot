//! just copy-pasted example from https://github.com/Lonami/grammers/blob/master/lib/grammers-client/examples/echo.rs

use futures_util::future::{select, Either};
use grammers_client::{Client, Config, InitParams, InputMessage, SignInError, types, Update};
use grammers_session::Session;
use grammers_tl_types;
use log;
// use simple_logger::SimpleLogger;
use std::env;
use std::pin::pin;
use std::time;
use grammers_tl_types::types::MessageEntityCustomEmoji;
use tokio::{runtime, task};


type Result = std::result::Result<(), Box<dyn std::error::Error>>;

const SESSION_FILE: &str = "echo.session";

async fn handle_update(client: Client, update: Update) -> Result {
    match update {
        Update::NewMessage(message) if message.outgoing() => {
            let chat = message.chat();
            let start = time::Instant::now();
            if message.text() != "/ping" {
                return Ok(());
            }
            println!("Responding to {}", chat.name());

            let text = "😀ust BOT\nPing";
            fn f_<T: Into<InputMessage>>(m: T) -> InputMessage {
                return m.into();
            }
            let mut input_message = f_(text);
            let entities = vec!(
                grammers_tl_types::enums::MessageEntity::CustomEmoji(
                    MessageEntityCustomEmoji{ offset: 0, length: 2, document_id: 5424780918776671920 }
                )
            );

            match client.send_message(&chat, input_message.fmt_entities(entities)).await {
                Err(E) => return Err(Box::try_from(E).unwrap()),
                Ok(msg) => {
                    let duration = start.elapsed();
                    let mut input_message = f_((text.to_owned() + &*format!(" {:?}", duration)).as_str());
                    let entities = vec!(
                        grammers_tl_types::enums::MessageEntity::CustomEmoji(
                            MessageEntityCustomEmoji{ offset: 0, length: 2, document_id: 5424780918776671920 }
                        )
                    );
                    client.edit_message(
                        &chat,
                        msg.id(),
                        input_message.fmt_entities(entities)
                    ).await?;
                }
            }
        }
        _ => {}
    }

    Ok(())
}



async fn async_main() -> Result {
    // SimpleLogger::new()
    //     .with_level(log::LevelFilter::Debug)
    //     .init()
    //     .unwrap();

    let api_id = env!("TG_ID").parse().expect("TG_ID invalid");
    let api_hash = env!("TG_HASH").to_string();
    let token = env::args().skip(1).next().expect("token missing");

    println!("Connecting to Telegram...");
    let client = Client::connect(Config {
        session: Session::load_file_or_create(SESSION_FILE)?,
        api_id,
        api_hash: api_hash.clone(),
        params: InitParams {
            // Fetch the updates we missed while we were offline
            catch_up: true,
            ..Default::default()
        },
    })
        .await?;
    println!("Connected!");

    if !client.is_authorized().await? {
        println!("Signing in...");
        fn ask_code_to_user() -> String {
            loop {
                let mut result = "".to_string();
                match std::io::stdin().read_line(&mut result) {
                    Ok(_) => break result.trim().parse().unwrap(),
                    Err(_) => println!("Invalid!")
                }
            }
        }

        let token = client.request_login_code(&*token).await?;
        let code = ask_code_to_user();

        let user = match client.sign_in(&token, &code).await {
            Ok(user) => user,
            Err(SignInError::PasswordRequired(_token)) => {
                println!("Please provide a password");
                let password = ask_code_to_user();
                client
                    .check_password(_token, password)
                    .await.unwrap()
            }
            Err(SignInError::SignUpRequired { terms_of_service: tos }) => panic!("Sign up required"),
            Err(err) => {
                println!("Failed to sign in as a user :(\n{}", err);
                return Err(err.into());
            }
        };

        client.session().save_to_file(SESSION_FILE)?;
        println!("Signed in as {}!", user.first_name());
    }

    println!("Waiting for messages...");

    loop {
        let update = {
            let exit = pin!(async { tokio::signal::ctrl_c().await });
            let upd = pin!(async { client.next_update().await });

            match select(exit, upd).await {
                Either::Left(_) => None,
                Either::Right((u, _)) => Some(u),
            }
        };

        let update = match update {
            None | Some(Ok(None)) => break,
            Some(u) => u?.unwrap(),
        };

        let handle = client.clone();
        task::spawn(async move {
            match handle_update(handle, update).await {
                Ok(_) => {}
                Err(e) => eprintln!("Error handling updates!: {}", e),
            }
        });
    }

    println!("Saving session file and exiting...");
    client.session().save_to_file(SESSION_FILE)?;
    Ok(())
}

fn main() -> Result {
    runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main())
}