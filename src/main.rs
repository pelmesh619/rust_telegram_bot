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
use grammers_client::types::User;
use grammers_tl_types::enums::MessageEntity;
use grammers_tl_types::types::{MessageEntityCustomEmoji, MessageEntityItalic, MessageEntityBold, MessageEntityUnderline};
use tokio::{runtime, task};
use regex::Regex;
use html_parser;


type Result = std::result::Result<(), Box<dyn std::error::Error>>;

const SESSION_FILE: &str = "echo.session";

fn ask_input_from_user() -> String {
    loop {
        let mut result = String::new();
        match std::io::stdin().read_line(&mut result) {
            Ok(_) => break result.trim().to_string(),
            Err(e) => println!("Some error raised: {}", e)
        }
    }
}

async fn sign_in(client: &Client, ) -> Result {
    if client.is_authorized().await? {
        return Ok(());
    }
    println!("Signing in...");

    let (token_or_phone, is_token) = loop {
        println!("Enter your phone number or bot token");
        let token_or_phone = ask_input_from_user();

        let phone_regex = Regex::new(r"^[\+]?[(]?[0-9]{3}[)]?[-\s\.]?[0-9]{3}[-\s\.]?[0-9]{4,6}$").unwrap();
        let token_regex = Regex::new(r"^\d+:[A-Za-z0-9_-]+$").unwrap();

        if token_regex.is_match(&*token_or_phone) {
            break (token_or_phone, true)
        } else if phone_regex.is_match(&*token_or_phone) {
            break (token_or_phone, false)
        } else {
            println!("This is not a phone number neither bot token. Try again.");
            continue
        }
    };

    if is_token {
        client.bot_sign_in(&token_or_phone).await?;
    } else {
        let token = client.request_login_code(&*token_or_phone).await?;
        println!("The code has been successfully requested. Enter the Telegram code:");
        let code = ask_input_from_user();

        let user = match client.sign_in(&token, &code).await {
            Ok(user) => user,
            Err(SignInError::PasswordRequired(_token)) => {
                let hint = _token.hint().unwrap_or("None");
                println!("You have a two-factor authentication, please enter your password: \n(Hint - {})", hint);
                loop {
                    let password = ask_input_from_user();
                    break match client.check_password(_token.clone(), password).await {
                        Ok(t) => t,
                        Err(e) => {
                            println!("It seems to be some error raised: {}", e);
                            continue;
                        },
                    }
                }
            }
            Err(SignInError::SignUpRequired { terms_of_service: tos }) =>
                panic!("It seems to be that this number does not have a Telegram account. Please sign up on other device and continue."),
            Err(err) => {
                println!("Failed to sign in as a user :(\n{}", err);
                return Err(err.into());
            }
        };
        println!("Signed in as {}!", user.first_name());
    }

    client.session().save_to_file(SESSION_FILE)?;
    return Ok(());
}


async fn handle_update(client: Client, update: Update) -> Result {
    match update {
        Update::NewMessage(message) if message.outgoing() => {
            let chat = message.chat();
            let start = time::Instant::now();
            if message.text() != "/ping" {
                return Ok(());
            }
            println!("Responding to {}", chat.name());

            let start_text = "[<emoji document_id=\"5424780918776671920\">😀</emoji>ust BOT]\n<b>Ping</b>";

            let (text, entities) = parse_entities(start_text);
            println!("{} {:?}", text, entities);
            fn f_<T: Into<InputMessage>>(m: T) -> InputMessage {
                return m.into();
            }
            let mut input_message = f_(text.clone());

            match client.send_message(&chat, input_message.fmt_entities(entities)).await {
                Err(e) => return Err(Box::try_from(e).unwrap()),
                Ok(msg) => {
                    let duration = start.elapsed();
                    let new_text = start_text.to_owned() + &*format!(" {:.3} ms", duration.as_secs_f64() * 1000f64).as_str();
                    let (new_text, entities) = parse_entities(new_text.as_str());
                    let mut input_message = f_(new_text.clone());
                    println!("{} {:?}", new_text, entities);


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
    // let token = env::args().skip(1).next().expect("token missing");

    println!("Connecting to Telegram...");
    let client = Client::connect(Config {
        session: Session::load_file_or_create(SESSION_FILE)?,
        api_id,
        api_hash: api_hash.clone(),
        params: InitParams {
            catch_up: false,
            ..Default::default()
        },
    })
        .await?;
    println!("Connected!");

    match sign_in(&client, ).await {
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
    client.session().save_to_file(SESSION_FILE)?;
    Ok(())
}

fn parse_entities(text: &str) -> (String, Vec<MessageEntity>) {
    let mut result = Vec::<MessageEntity>::new();
    let mut new_text = String::new();

    let r = html_parser::Dom::parse(text).unwrap();

    println!("{}", r.to_json_pretty().unwrap());

    fn rec_parse(cur: &html_parser::Node, offset: usize, result: &mut Vec<MessageEntity>, new_text: &mut String) -> usize {
        if let Some(t) = cur.text() {
            *new_text += t;
            return t.encode_utf16().collect::<Vec<_>>().len()
        }

        match cur.element() {
            Some(e) => {
                let mut len = 0;

                for i in &e.children {
                    len += rec_parse(&i, offset + len, result, new_text);
                }
                let entity = match e.name.as_str() {
                    "i" => {
                        Some(MessageEntity::Italic(
                            MessageEntityItalic{ offset: offset as i32, length: len as i32 })
                        )
                    }
                    "b" => {
                        Some(MessageEntity::Bold(
                            MessageEntityBold{ offset: offset as i32, length: len as i32 })
                        )
                    }
                    "u" => {
                        Some(MessageEntity::Underline(
                            MessageEntityUnderline{ offset: offset as i32, length: len as i32 })
                        )
                    }
                    "emoji" => {
                        if let Some(option) = e.attributes.get("document_id") {
                            if let Some(document_id) = option {
                                Some(MessageEntity::CustomEmoji(
                                    MessageEntityCustomEmoji { offset: offset as i32, length: len as i32, document_id: document_id.parse::<i64>().unwrap_or(0) })
                                )
                            } else { None }
                        } else if let Some(document_id) = &e.id {
                            Some(MessageEntity::CustomEmoji(
                                MessageEntityCustomEmoji { offset: offset as i32, length: len as i32, document_id: document_id.parse::<i64>().unwrap_or(0) })
                            )
                        } else { None }
                    }
                    _ => { None }
                };
                if let Some(entity) = entity {
                    if entity.length() > 0{
                        result.push(entity);
                    }
                }
                len
            }
            None => { 0 }
        }

    }

    let mut offset = 0usize;
    for i in r.children {
        offset += rec_parse(&i, offset, &mut result, &mut new_text);
    }

    (new_text.parse().unwrap(), result)
}


fn main() -> Result {
    runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main())

    // println!("{:?}", parse_entities("<b><i>huhuhu</i>joijio</b>ojijio<u><emoji id=\"3108380\">ojefjof</emoji>efjoeoj</u>wwwd<a href=\"gjojife\">gyggy</a>"));
    //
    // Ok(())
}