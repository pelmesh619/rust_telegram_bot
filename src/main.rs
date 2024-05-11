use futures_util::future::{select, Either};
use grammers_client::{Client, Config, InitParams, Update};
use grammers_session::Session;
use std::env;
use std::pin::pin;
use std::time;
use grammers_client::types::{Message};
use tokio::{runtime, task};
use rand;

mod utils;
mod client_methods;


/// just copy-pasted example from https://github.com/Lonami/grammers/blob/master/lib/grammers-client/examples/echo.rs


type Result = std::result::Result<(), Box<dyn std::error::Error>>;

async fn filter1(_client: &Client, message: &Message) -> bool {
    message.text() == "/test"
}
async fn handler1(_client: &Client, message: &Message) -> Result {
    match client_methods::reply_to_message(message, "Bot is <emoji id=\"5424780918776671920\">😀</emoji><b>USTing</b>").await {
        Err(e) => return Err(Box::try_from(e).unwrap()),
        Ok(_) => { Ok(()) }
    }
}

async fn filter2(_client: &Client, message: &Message) -> bool {
    message.outgoing() && message.text() == "/ping"
}
async fn handler2(_client: &Client, message: &Message) -> Result {
    let start = time::Instant::now();
    let start_text = "[<emoji document_id=\"5424780918776671920\">😀</emoji>ust BOT]\n<b>Ping</b>";

    match client_methods::reply_to_message(&message, "[<emoji document_id=\"5424780918776671920\">😀</emoji>ust BOT]\n<b>Ping</b>").await {
        Err(e) => return Err(Box::try_from(e).unwrap()),
        Ok(msg) => {
            let duration = start.elapsed();
            let new_text = start_text.to_owned() + &*format!(" {:.3} ms", duration.as_secs_f64() * 1000f64).as_str();

            client_methods::edit_message(&msg, new_text.as_str()).await?;
            Ok(())
        }
    }
}

const SECRET_CHAT: i64 = 0;

async fn filter3(_client: &Client, message: &Message) -> bool {
    !message.outgoing() && !message.text().is_empty() && message.chat().id() == SECRET_CHAT && rand::random::<f32>() < 0.2
}
async fn handler3(client: &Client, message: &Message) -> Result {
    match client_methods::send_message(
        client, message.chat(),
        format!("<i>Как однажды сказал один мудрый человек</i>\n<blockquote>{}</blockquote>", message.text()).as_str()
    ).await {
        Err(e) => return Err(Box::try_from(e).unwrap()),
        Ok(_) => { Ok(()) }
    }
}

async fn handle_update(client: Client, update: Update) -> Result {
    match update {
        Update::NewMessage(message) => {
            if filter1(&client, &message).await {
                return handler1(&client, &message).await;
            }
            if filter2(&client, &message).await {
                return handler2(&client, &message).await;
            }
            if filter3(&client, &message).await {
                return handler3(&client, &message).await;
            }
        }
        _ => {}
    }

    Ok(())
}


async fn async_main() -> Result {
    let api_id = env!("TG_ID").parse().expect("TG_ID invalid");
    let api_hash = env!("TG_HASH").to_string();

    println!("Connecting to Telegram...");
    let client = Client::connect(Config {
        session: Session::load_file_or_create(client_methods::SESSION_FILE)?,
        api_id,
        api_hash: api_hash.clone(),
        params: InitParams {
            catch_up: false,
            ..Default::default()
        },
    }).await?;
    println!("Connected!");

    match client_methods::sign_in(&client, ).await {
        Err(e) => return Err(e),
        Ok(_) => (),
    };

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
    client.session().save_to_file(client_methods::SESSION_FILE)?;
    Ok(())
}




fn main() -> Result {
    runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main())
}